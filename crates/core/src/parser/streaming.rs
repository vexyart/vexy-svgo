// this_file: crates/core/src/parser/streaming.rs

use std::collections::HashMap;
use std::io::BufRead;

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::ast::{Document, Element, Node};
use crate::error::VexyError;
use crate::parser::config::StreamingConfig;
use crate::parser::entities::{expand_entities_in_text, parse_entities_from_doctype};
use crate::parser::util::text_elements;

/// Streaming parser for large SVG documents
pub struct StreamingParser<R: BufRead> {
    reader:               Reader<R>,
    config:               StreamingConfig,
    preserve_whitespace:  bool,
    preserve_comments:    bool,
    expand_entities:      bool,
    #[allow(dead_code)]
    file_path:            Option<String>,
    current_depth:        usize,
    bytes_processed:      usize,
    current_memory_usage: usize,
}

impl<R: BufRead> StreamingParser<R> {
    pub fn new(
        reader: R,
        config: StreamingConfig,
        preserve_whitespace: bool,
        preserve_comments: bool,
        expand_entities: bool,
        file_path: Option<String>,
    ) -> Self {
        let xml_reader = Reader::from_reader(reader);

        // Configure reader for quick-xml 0.31
        // Note: In quick-xml 0.31, configuration is done differently
        // The trim settings are controlled by the parser, not the reader
        // expand_empty_elements is handled automatically
        // check_end_names is enabled by default

        Self {
            reader: xml_reader,
            config,
            preserve_whitespace,
            preserve_comments,
            expand_entities,
            file_path,
            current_depth: 0,
            bytes_processed: 0,
            current_memory_usage: 0,
        }
    }

    /// Track memory usage and return error if limit exceeded
    fn track_memory(&mut self, size: usize) -> Result<(), VexyError> {
        if let Some(limit) = self.config.memory_limit {
            self.current_memory_usage = self.current_memory_usage.saturating_add(size);
            if self.current_memory_usage > limit {
                return Err(VexyError::Parse(crate::parser::error::ParseError::StructureError(format!(
                    "Memory limit exceeded: {} bytes (limit: {})",
                    self.current_memory_usage, limit
                ))));
            }
        }
        Ok(())
    }

    /// Parse the document with streaming optimizations
    pub fn parse(&mut self) -> Result<Document<'static>, VexyError> {
        let mut document = Document::new();
        let mut element_stack: Vec<Element<'static>> = Vec::new();
        let mut current_element: Option<Element<'static>> = None;
        let mut found_root = false;
        let mut entities: HashMap<String, String> = HashMap::new();
        let mut element_name_stack: Vec<String> = Vec::new();
        let mut buf = Vec::with_capacity(self.config.buffer_size);

        loop {
            buf.clear();

            // Check depth limit to prevent stack overflow
            if self.current_depth > self.config.max_depth {
                return Err(VexyError::Parse(crate::parser::error::ParseError::StructureError(format!(
                    "Maximum parsing depth exceeded: {}",
                    self.config.max_depth
                ))));
            }

            match self.reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    self.current_depth += 1;
                    let element = self.parse_start_element(e, &entities)?;
                    element_name_stack.push(element.name.to_string());

                    if current_element.is_none() {
                        current_element = Some(element);
                    } else {
                        if let Some(elem) = current_element.take() {
                            element_stack.push(elem);
                        }
                        current_element = Some(element);
                    }
                },
                Ok(Event::End(_)) => {
                    self.current_depth = self.current_depth.saturating_sub(1);
                    element_name_stack.pop();

                    if let Some(finished_element) = current_element.take() {
                        if let Some(mut parent) = element_stack.pop() {
                            parent.add_child(Node::Element(finished_element));
                            current_element = Some(parent);
                        } else {
                            document.root = finished_element;
                            found_root = true;
                        }
                    }
                },
                Ok(Event::Empty(ref e)) => {
                    let element = self.parse_start_element(e, &entities)?;

                    if let Some(ref mut parent) = current_element {
                        parent.add_child(Node::Element(element));
                    } else {
                        document.root = element;
                        found_root = true;
                    }
                },
                Ok(Event::Text(ref e)) => {
                    let text_str = match std::str::from_utf8(e.as_ref()) {
                        Ok(s) => s,
                        Err(_) => continue, // Skip invalid UTF-8 in streaming mode
                    };

                    // Check for large text nodes
                    if let Some(threshold) = self.config.large_text_threshold {
                        if text_str.len() > threshold {
                            // Skip or truncate very large text nodes to preserve memory
                            if let Some(ref mut element) = current_element {
                                let truncated = format!(
                                    "{}... [truncated {} bytes]",
                                    &text_str[..threshold.min(100)],
                                    text_str.len() - threshold.min(100),
                                );
                                element.add_child(Node::Text(truncated.into()));
                            }
                            continue;
                        }
                    }

                    let text = quick_xml::escape::unescape(text_str).unwrap_or(std::borrow::Cow::Borrowed(text_str));
                    let mut text_content = text.into_owned();

                    if self.expand_entities && !entities.is_empty() {
                        text_content = expand_entities_in_text(&text_content, &entities);
                    }

                    let should_preserve_whitespace = self.preserve_whitespace
                        || element_name_stack
                            .last()
                            .map(|name| text_elements().contains(name.as_str()))
                            .unwrap_or(false);

                    if should_preserve_whitespace || !text_content.trim().is_empty() {
                        if let Some(ref mut element) = current_element {
                            // Track memory for text node
                            self.track_memory(text_content.len() + std::mem::size_of::<Node>())?;
                            element.add_child(Node::Text(text_content.into()));
                        }
                    }
                },
                Ok(Event::Comment(ref e)) => {
                    if self.preserve_comments {
                        let comment = match std::str::from_utf8(e.as_ref()) {
                            Ok(s) => s.to_string(),
                            Err(_) => continue, // Skip invalid UTF-8
                        };

                        let comment_size = comment.len() + std::mem::size_of::<Node>();
                        self.track_memory(comment_size)?;

                        if let Some(ref mut element) = current_element {
                            element.add_child(Node::Comment(comment.into()));
                        } else if !found_root {
                            document.prologue.push(Node::Comment(comment.into()));
                        } else {
                            document.epilogue.push(Node::Comment(comment.into()));
                        }
                    }
                },
                Ok(Event::CData(ref e)) => {
                    let cdata = match std::str::from_utf8(e.as_ref()) {
                        Ok(s) => s.to_string(),
                        Err(_) => continue, // Skip invalid UTF-8
                    };

                    self.track_memory(cdata.len() + std::mem::size_of::<Node>())?;

                    if let Some(ref mut element) = current_element {
                        element.add_child(Node::CData(cdata.into()));
                    }
                },
                Ok(Event::PI(ref e)) => {
                    let pi_data = match std::str::from_utf8(e.as_ref()) {
                        Ok(s) => s,
                        Err(_) => continue, // Skip invalid UTF-8
                    };

                    let parts: Vec<&str> = pi_data.splitn(2, ' ').collect();
                    let target = parts[0].to_string();
                    let data = parts.get(1).unwrap_or(&"").to_string();

                    self.track_memory(target.len() + data.len() + std::mem::size_of::<Node>())?;

                    let pi_node = Node::ProcessingInstruction {
                        target: target.into(),
                        data:   data.into(),
                    };

                    if let Some(ref mut element) = current_element {
                        element.add_child(pi_node);
                    } else if !found_root {
                        document.prologue.push(pi_node);
                    } else {
                        document.epilogue.push(pi_node);
                    }
                },
                Ok(Event::Decl(ref e)) => {
                    if let Ok(version) = e.version() {
                        document.metadata.version = Some(String::from_utf8_lossy(&version).into());
                    }
                    if let Some(Ok(enc)) = e.encoding() {
                        document.metadata.encoding = Some(String::from_utf8_lossy(&enc).into());
                    }
                },
                Ok(Event::DocType(ref e)) => {
                    let doctype = match std::str::from_utf8(e.as_ref()) {
                        Ok(s) => s.to_string(),
                        Err(_) => continue, // Skip invalid UTF-8
                    };

                    if self.expand_entities {
                        parse_entities_from_doctype(&doctype, &mut entities);
                    }

                    if !found_root {
                        self.track_memory(doctype.len() + std::mem::size_of::<Node>())?;
                        document.prologue.push(Node::DocType(doctype.into()));
                    }
                },
                Ok(Event::Eof) => break,
                // GeneralRef events are ignored in streaming mode
                // Ok(Event::GeneralRef(_)) => {
                //     // Ignore general references in streaming mode
                // }
                Err(e) => {
                    // In streaming mode, we're more tolerant of errors
                    // but still track position for debugging
                    let byte_pos = self.reader.buffer_position();
                    return Err(VexyError::Parse(crate::parser::error::ParseError::StructureError(format!(
                        "Streaming parse error at byte {byte_pos}: {e}"
                    ))));
                },
            }

            // Track bytes processed for progress reporting
            self.bytes_processed = self.reader.buffer_position();
        }

        if current_element.is_none() && document.root.name.is_empty() {
            return Err(VexyError::Parse(crate::parser::error::ParseError::StructureError(
                "No root element found in document".to_string(),
            )));
        }

        // Optimize memory usage after parsing
        if !self.config.lazy_loading {
            document.root.optimize_memory();
        }

        Ok(document)
    }

    /// Get the number of bytes processed so far
    pub fn bytes_processed(&self) -> usize {
        self.bytes_processed
    }

    /// Get the current parsing depth
    pub fn current_depth(&self) -> usize {
        self.current_depth
    }

    // Helper methods from the main parser
    fn parse_start_element(
        &mut self,
        start: &quick_xml::events::BytesStart<'_>,
        entities: &HashMap<String, String>,
    ) -> Result<Element<'static>, VexyError> {
        let name = match std::str::from_utf8(start.name().as_ref()) {
            Ok(n) => n.to_string(),
            Err(_) => "unknown".to_string(), // More tolerant in streaming mode
        };

        // Track memory for element structure and name
        self.track_memory(std::mem::size_of::<Element>() + name.len())?;

        let mut element = if self.config.lazy_loading {
            Element::new(std::borrow::Cow::Owned(name))
        } else {
            // Pre-allocate capacity based on typical SVG element patterns
            let expected_children = match name.as_str() {
                "svg" | "g" | "defs" => 10,      // Container elements
                "path" | "rect" | "circle" => 0, // Leaf elements
                _ => 2,                          // Default
            };
            Element::with_capacity(std::borrow::Cow::Owned(name), expected_children)
        };

        // Parse attributes with error tolerance and zero-copy optimizations
        for attr_result in start.attributes() {
            match attr_result {
                Ok(attr) => {
                    // 1. Zero-copy key parsing
                    let key_str = match std::str::from_utf8(attr.key.as_ref()) {
                        Ok(k) => k,
                        Err(_) => continue, // Skip invalid UTF-8 keys
                    };

                    // 2. Zero-copy value parsing (borrowed if no entities)
                    // unescape_value returns Cow<'a, str>, avoiding allocation if possible
                    match attr.unescape_value() {
                        Ok(value) => {
                            let mut final_value = value;

                            // 3. Handle custom entities if needed
                            if self.expand_entities && !entities.is_empty() {
                                // Only allocate if we actually perform expansion
                                // Check if value contains '&' to avoid unnecessary processing
                                if final_value.contains('&') {
                                    let expanded = expand_entities_in_text(&final_value, entities);
                                    if expanded != final_value {
                                        final_value = std::borrow::Cow::Owned(expanded);
                                    }
                                }
                            }

                            // 4. Insert into Element
                            // Since Document is 'static, we must own the data eventually,
                            // but we avoid intermediate String allocations.
                            // We use into_owned() to consume the Cow directly.
                            let value_owned = final_value.into_owned();

                            // Track memory for attribute (approximate overhead)
                            // Key string + Value string + entry overhead
                            self.track_memory(key_str.len() + value_owned.len() + 64)?;

                            // Track namespaces for namespace-aware logic
                            if key_str == "xmlns" {
                                element.namespaces.insert("".into(), value_owned.clone().into());
                            } else if let Some(ns_name) = key_str.strip_prefix("xmlns:") {
                                element.namespaces.insert(ns_name.into(), value_owned.clone().into());
                            }

                            element.set_attr(key_str.to_string(), value_owned);
                        },
                        Err(_) => continue, // Skip invalid attribute values
                    }
                },
                Err(_) => continue, // Skip malformed attributes
            }
        }

        Ok(element)
    }
}
