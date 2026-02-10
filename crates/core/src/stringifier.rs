// this_file: crates/core/src/stringifier.rs

//! Convert AST back to SVG string with performance optimizations
//!
//! This module provides efficient stringification of SVG documents with:
//! - Pre-allocated string buffers to minimize reallocations
//! - Optimized escape functions that avoid unnecessary allocations
//! - Configurable formatting options (pretty print, minified)
//! - Streaming output support for large documents

use std::fmt::Write;

use crate::ast::{Document, Element, Node};
use crate::error::VexyError;

/// Configuration options for stringification
#[derive(Debug, Clone)]
pub struct StringifyConfig {
    /// Whether to pretty-print with indentation
    pub pretty:           bool,
    /// Indentation string (e.g., "  " or "    ")
    pub indent:           String,
    /// Whether to add newlines
    pub newlines:         bool,
    /// Whether to quote attribute values (always true for SVG)
    pub quote_attrs:      bool,
    /// Whether to self-close empty elements
    pub self_close:       bool,
    /// Initial buffer capacity for better performance
    pub initial_capacity: usize,
}

impl Default for StringifyConfig {
    fn default() -> Self {
        Self {
            pretty:           false,
            indent:           "  ".to_string(),
            newlines:         false,
            quote_attrs:      true,
            self_close:       true,
            initial_capacity: 4096, // 4KB initial buffer
        }
    }
}

impl StringifyConfig {
    /// Create a minified configuration (default)
    pub fn minified() -> Self {
        Self::default()
    }

    /// Create a pretty-print configuration
    pub fn pretty() -> Self {
        Self {
            pretty: true,
            newlines: true,
            ..Default::default()
        }
    }
}

/// Convert a Document to an SVG string with default settings
pub fn stringify(document: &Document) -> Result<String, VexyError> {
    stringify_with_config(document, &StringifyConfig::default())
}

/// Convert a Document to an SVG string with custom configuration
pub fn stringify_with_config(document: &Document, config: &StringifyConfig) -> Result<String, VexyError> {
    // Pre-allocate string with estimated capacity
    let estimated_size = estimate_document_size(document);
    let mut output = String::with_capacity(estimated_size.max(config.initial_capacity));

    // Write XML declaration if present
    if let Some(ref version) = document.metadata.version {
        write!(output, "<?xml version=\"{version}\"")?;
        if let Some(ref encoding) = document.metadata.encoding {
            write!(output, " encoding=\"{encoding}\"")?;
        }
        if let Some(ref standalone) = document.metadata.standalone {
            write!(output, " standalone=\"{standalone}\"")?;
        }
        output.push_str("?>");
        if config.newlines {
            output.push('\n');
        }
    }

    // Write prologue nodes
    for node in &document.prologue {
        stringify_node(node, &mut output, config, 0)?;
        if config.newlines && !matches!(node, Node::Text(_)) {
            output.push('\n');
        }
    }

    // Write root element
    stringify_element(&document.root, &mut output, config, 0)?;

    // Write epilogue nodes
    if !document.epilogue.is_empty() && config.newlines {
        output.push('\n');
    }
    for node in &document.epilogue {
        stringify_node(node, &mut output, config, 0)?;
        if config.newlines && !matches!(node, Node::Text(_)) {
            output.push('\n');
        }
    }

    Ok(output)
}

/// Estimate the size of the document for pre-allocation
fn estimate_document_size(document: &Document) -> usize {
    // Basic estimation: element overhead + attributes + text content
    estimate_element_size(&document.root)
        + document.prologue.iter().map(estimate_node_size).sum::<usize>()
        + document.epilogue.iter().map(estimate_node_size).sum::<usize>()
}

fn estimate_element_size(element: &Element) -> usize {
    let mut size = element.name.len() * 2 + 5; // <name> + </name>

    // Attributes
    for (name, value) in &element.attributes {
        size += name.len() + value.len() + 4; // name="value"
    }

    // Namespaces
    for (prefix, uri) in &element.namespaces {
        if prefix.is_empty() {
            size += 7 + uri.len(); // xmlns="uri"
        } else {
            size += 7 + prefix.len() + uri.len(); // xmlns:prefix="uri"
        }
    }

    // Children
    size += element.children.iter().map(estimate_node_size).sum::<usize>();

    size
}

fn estimate_node_size(node: &Node) -> usize {
    match node {
        Node::Element(e) => estimate_element_size(e),
        Node::Text(t) => t.len(),
        Node::Comment(c) => c.len() + 7, // <!-- -->
        Node::CData(c) => c.len() + 12,  // <![CDATA[]]>
        Node::ProcessingInstruction { target, data } => target.len() + data.len() + 5, // <? ?>
        Node::DocType(d) => d.len() + 11, // <!DOCTYPE >
    }
}

/// Stringify an element with optimized performance
fn stringify_element(
    element: &Element,
    output: &mut String,
    config: &StringifyConfig,
    depth: usize,
) -> Result<(), VexyError> {
    // Indentation (skip when indentation already exists in preserved whitespace)
    if config.pretty && depth > 0 {
        let line_start = output.rfind('\n').map_or(0, |index| index + 1);
        let line_prefix = &output[line_start..];
        let has_existing_indent = !line_prefix.is_empty() && line_prefix.chars().all(|c| c == ' ');

        if !has_existing_indent {
            for _ in 0..depth {
                output.push_str(&config.indent);
            }
        }
    }

    // Opening tag
    output.push('<');
    output.push_str(&element.name);

    // Attributes
    for (name, value) in &element.attributes {
        output.push(' ');
        output.push_str(name);
        if value.is_empty() {
            output.push('=');
            if config.quote_attrs {
                output.push_str("\"\"");
            }
            continue;
        }
        output.push('=');
        if value.starts_with('{') && value.ends_with('}') {
            output.push_str(value);
        } else if config.quote_attrs {
            output.push('"');
            output.push_str(value);
            output.push('"');
        } else {
            output.push_str(value);
        }
    }

    // Emit namespaces tracked outside attributes (e.g. programmatic AST construction).
    for (prefix, uri) in &element.namespaces {
        let xmlns_key = if prefix.is_empty() {
            "xmlns".to_string()
        } else {
            format!("xmlns:{prefix}")
        };

        if element.attributes.contains_key(xmlns_key.as_str()) {
            continue;
        }

        output.push(' ');
        output.push_str(&xmlns_key);
        output.push('=');
        if config.quote_attrs {
            output.push('"');
            output.push_str(uri);
            output.push('"');
        } else {
            output.push_str(uri);
        }
    }

    if element.children.is_empty() && config.self_close {
        // Self-closing tag
        output.push_str("/>");
    } else {
        // Opening tag close
        output.push('>');

        let has_element_children = element.children.iter().any(|n| matches!(n, Node::Element(_)));
        let has_non_empty_text = element.children.iter().any(|n| match n {
            Node::Text(t) => !t.trim().is_empty(),
            _ => false,
        });
        let starts_with_formatting_text = matches!(
            element.children.first(),
            Some(Node::Text(text)) if text.trim().is_empty() && text.contains('\n')
        );
        let ends_with_formatting_text = matches!(
            element.children.last(),
            Some(Node::Text(text)) if text.trim().is_empty() && text.contains('\n')
        );
        let inline_text_element = element.name == "text"
            && !has_element_children
            && element.children.len() == 1
            && matches!(
                element.children.first(),
                Some(Node::Text(t)) if !t.trim().is_empty() && !t.contains('\n') && t.as_ref() == t.trim()
            );
        let has_content_children = has_element_children || has_non_empty_text;
        let is_formatting_text = |node: &Node| matches!(node, Node::Text(text) if text.trim().is_empty());

        if has_content_children && config.newlines && !inline_text_element && !starts_with_formatting_text {
            output.push('\n');
        }

        // Children
        for (i, child) in element.children.iter().enumerate() {
            match child {
                Node::Element(_) => {
                    stringify_node(child, output, config, depth + 1)?;
                    let next_is_formatting = matches!(
                        element.children.get(i + 1),
                        Some(next_child) if is_formatting_text(next_child)
                    );
                    if config.newlines && i < element.children.len() - 1 && !next_is_formatting {
                        output.push('\n');
                    }
                },
                Node::Text(t) if !t.trim().is_empty() => {
                    let text_depth = if inline_text_element {
                        0
                    } else if element.name == "textPath" {
                        depth
                    } else {
                        depth + 1
                    };
                    stringify_node(child, output, config, text_depth)?;
                    let next_is_formatting = matches!(
                        element.children.get(i + 1),
                        Some(next_child) if is_formatting_text(next_child)
                    );
                    if config.newlines && !inline_text_element && i < element.children.len() - 1 && !next_is_formatting
                    {
                        output.push('\n');
                    }
                },
                Node::Text(t) => {
                    if config.pretty && config.newlines {
                        output.push_str(t);
                    }
                },
                _ => {
                    stringify_node(child, output, config, depth + 1)?;
                },
            }
        }

        if has_content_children && config.newlines && !inline_text_element && !ends_with_formatting_text {
            output.push('\n');
            if config.pretty {
                for _ in 0..depth {
                    output.push_str(&config.indent);
                }
            }
        }

        // Closing tag
        output.push_str("</");
        output.push_str(&element.name);
        output.push('>');
    }

    Ok(())
}

/// Stringify a node
fn stringify_node(node: &Node, output: &mut String, config: &StringifyConfig, depth: usize) -> Result<(), VexyError> {
    match node {
        Node::Element(e) => stringify_element(e, output, config, depth),
        Node::Text(t) => {
            // Check if text should be indented
            let should_indent = config.pretty && depth > 0 && !t.trim().is_empty();
            if should_indent {
                for _ in 0..depth {
                    output.push_str(&config.indent);
                }
            }
            let text = if config.pretty { t.trim() } else { t.as_ref() };
            escape_text_to(text, output);
            Ok(())
        },
        Node::Comment(c) => {
            if config.pretty && depth > 0 {
                for _ in 0..depth {
                    output.push_str(&config.indent);
                }
            }
            output.push_str("<!--");
            output.push_str(c);
            output.push_str("-->");
            Ok(())
        },
        Node::CData(c) => {
            output.push_str("<![CDATA[");
            output.push_str(c);
            output.push_str("]]>");
            Ok(())
        },
        Node::ProcessingInstruction { target, data } => {
            output.push_str("<?");
            output.push_str(target);
            if !data.is_empty() {
                output.push(' ');
                output.push_str(data);
            }
            output.push_str("?>");
            Ok(())
        },
        Node::DocType(doctype) => {
            output.push_str("<!DOCTYPE ");
            output.push_str(doctype);
            output.push('>');
            Ok(())
        },
    }
}

/// Escape text content, writing directly to output to avoid allocation
fn escape_text_to(s: &str, output: &mut String) {
    // Fast path: if no special characters, append directly
    if !s.contains(&['&', '<', '>', '"', '\''][..]) {
        output.push_str(s);
        return;
    }

    // Reserve additional capacity for escapes
    output.reserve(s.len() + 10);

    for ch in s.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(ch),
        }
    }
}

/// Escape attribute content, writing directly to output to avoid allocation
#[cfg(test)]
fn escape_attribute_to(s: &str, output: &mut String) {
    // Fast path: if no special characters, append directly
    if !s.contains(&['&', '<', '>', '"', '\''][..]) {
        output.push_str(s);
        return;
    }

    // Reserve additional capacity for escapes
    output.reserve(s.len() + 10);

    for ch in s.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(ch),
        }
    }
}

/// Streaming stringifier for very large documents
pub struct StreamingStringifier<W: std::io::Write> {
    writer: W,
    config: StringifyConfig,
}

impl<W: std::io::Write> StreamingStringifier<W> {
    /// Create a new streaming stringifier
    pub fn new(writer: W, config: StringifyConfig) -> Self {
        Self { writer, config }
    }

    /// Stringify a document to the writer
    pub fn stringify(&mut self, document: &Document) -> Result<(), VexyError> {
        // Write XML declaration
        if let Some(ref version) = document.metadata.version {
            write!(self.writer, "<?xml version=\"{version}\"")?;
            if let Some(ref encoding) = document.metadata.encoding {
                write!(self.writer, " encoding=\"{encoding}\"")?;
            }
            if let Some(ref standalone) = document.metadata.standalone {
                write!(self.writer, " standalone=\"{standalone}\"")?;
            }
            self.writer.write_all(b"?>")?;
            if self.config.newlines {
                self.writer.write_all(b"\n")?;
            }
        }

        // Write prologue
        for node in &document.prologue {
            self.stringify_node(node, 0)?;
            if self.config.newlines && !matches!(node, Node::Text(_)) {
                self.writer.write_all(b"\n")?;
            }
        }

        // Write root element
        self.stringify_element(&document.root, 0)?;

        // Write epilogue
        if !document.epilogue.is_empty() && self.config.newlines {
            self.writer.write_all(b"\n")?;
        }
        for node in &document.epilogue {
            self.stringify_node(node, 0)?;
            if self.config.newlines && !matches!(node, Node::Text(_)) {
                self.writer.write_all(b"\n")?;
            }
        }

        self.writer.flush()?;
        Ok(())
    }

    fn write_indent(&mut self, depth: usize) -> Result<(), VexyError> {
        if self.config.pretty {
            for _ in 0..depth {
                self.writer.write_all(self.config.indent.as_bytes())?;
            }
        }
        Ok(())
    }

    fn write_escaped_text(&mut self, s: &str) -> Result<(), VexyError> {
        if !s.contains(&['&', '<', '>', '"', '\''][..]) {
            self.writer.write_all(s.as_bytes())?;
            return Ok(());
        }

        for ch in s.chars() {
            match ch {
                '&' => self.writer.write_all(b"&amp;")?,
                '<' => self.writer.write_all(b"&lt;")?,
                '>' => self.writer.write_all(b"&gt;")?,
                '"' => self.writer.write_all(b"&quot;")?,
                '\'' => self.writer.write_all(b"&apos;")?,
                _ => {
                    let mut buf = [0u8; 4];
                    let encoded = ch.encode_utf8(&mut buf);
                    self.writer.write_all(encoded.as_bytes())?;
                },
            }
        }

        Ok(())
    }

    fn stringify_element(&mut self, element: &Element, depth: usize) -> Result<(), VexyError> {
        if self.config.pretty && depth > 0 {
            self.write_indent(depth)?;
        }

        self.writer.write_all(b"<")?;
        self.writer.write_all(element.name.as_bytes())?;

        for (name, value) in &element.attributes {
            self.writer.write_all(b" ")?;
            self.writer.write_all(name.as_bytes())?;
            self.writer.write_all(b"=")?;

            if self.config.quote_attrs {
                self.writer.write_all(b"\"")?;
                self.writer.write_all(value.as_bytes())?;
                self.writer.write_all(b"\"")?;
            } else {
                self.writer.write_all(value.as_bytes())?;
            }
        }

        for (prefix, uri) in &element.namespaces {
            let xmlns_key = if prefix.is_empty() {
                "xmlns".to_string()
            } else {
                format!("xmlns:{prefix}")
            };

            if element.attributes.contains_key(xmlns_key.as_str()) {
                continue;
            }

            self.writer.write_all(b" ")?;
            self.writer.write_all(xmlns_key.as_bytes())?;
            self.writer.write_all(b"=")?;
            if self.config.quote_attrs {
                self.writer.write_all(b"\"")?;
                self.writer.write_all(uri.as_bytes())?;
                self.writer.write_all(b"\"")?;
            } else {
                self.writer.write_all(uri.as_bytes())?;
            }
        }

        if element.children.is_empty() && self.config.self_close {
            self.writer.write_all(b"/>")?;
            return Ok(());
        }

        self.writer.write_all(b">")?;

        let has_element_children = element.children.iter().any(|n| matches!(n, Node::Element(_)));
        let has_non_empty_text = element.children.iter().any(|n| match n {
            Node::Text(t) => !t.trim().is_empty(),
            _ => false,
        });
        let starts_with_formatting_text = matches!(
            element.children.first(),
            Some(Node::Text(text)) if text.trim().is_empty() && text.contains('\n')
        );
        let ends_with_formatting_text = matches!(
            element.children.last(),
            Some(Node::Text(text)) if text.trim().is_empty() && text.contains('\n')
        );
        let inline_text_element = element.name == "text"
            && !has_element_children
            && element.children.len() == 1
            && matches!(
                element.children.first(),
                Some(Node::Text(t)) if !t.trim().is_empty() && !t.contains('\n') && t.as_ref() == t.trim()
            );
        let has_content_children = has_element_children || has_non_empty_text;
        let is_formatting_text = |node: &Node| matches!(node, Node::Text(text) if text.trim().is_empty());

        if has_content_children && self.config.newlines && !inline_text_element && !starts_with_formatting_text {
            self.writer.write_all(b"\n")?;
        }

        for (i, child) in element.children.iter().enumerate() {
            match child {
                Node::Element(_) => {
                    self.stringify_node(child, depth + 1)?;
                    let next_is_formatting = matches!(
                        element.children.get(i + 1),
                        Some(next_child) if is_formatting_text(next_child)
                    );
                    if self.config.newlines && i < element.children.len() - 1 && !next_is_formatting {
                        self.writer.write_all(b"\n")?;
                    }
                },
                Node::Text(t) if !t.trim().is_empty() => {
                    let text_depth = if inline_text_element {
                        0
                    } else if element.name == "textPath" {
                        depth
                    } else {
                        depth + 1
                    };
                    self.stringify_node(child, text_depth)?;
                    let next_is_formatting = matches!(
                        element.children.get(i + 1),
                        Some(next_child) if is_formatting_text(next_child)
                    );
                    if self.config.newlines
                        && !inline_text_element
                        && i < element.children.len() - 1
                        && !next_is_formatting
                    {
                        self.writer.write_all(b"\n")?;
                    }
                },
                Node::Text(t) => {
                    if self.config.pretty && self.config.newlines {
                        self.writer.write_all(t.as_bytes())?;
                    }
                },
                _ => self.stringify_node(child, depth + 1)?,
            }
        }

        if has_content_children && self.config.newlines && !inline_text_element && !ends_with_formatting_text {
            self.writer.write_all(b"\n")?;
            if self.config.pretty {
                self.write_indent(depth)?;
            }
        }

        self.writer.write_all(b"</")?;
        self.writer.write_all(element.name.as_bytes())?;
        self.writer.write_all(b">")?;

        Ok(())
    }

    fn stringify_node(&mut self, node: &Node, depth: usize) -> Result<(), VexyError> {
        match node {
            Node::Element(e) => self.stringify_element(e, depth),
            Node::Text(t) => {
                let should_indent = self.config.pretty && depth > 0 && !t.trim().is_empty();
                if should_indent {
                    self.write_indent(depth)?;
                }
                let text = if self.config.pretty {
                    t.trim()
                } else {
                    t.as_ref()
                };
                self.write_escaped_text(text)
            },
            Node::Comment(c) => {
                if self.config.pretty && depth > 0 {
                    self.write_indent(depth)?;
                }
                self.writer.write_all(b"<!--")?;
                self.writer.write_all(c.as_bytes())?;
                self.writer.write_all(b"-->")?;
                Ok(())
            },
            Node::CData(c) => {
                self.writer.write_all(b"<![CDATA[")?;
                self.writer.write_all(c.as_bytes())?;
                self.writer.write_all(b"]]>")?;
                Ok(())
            },
            Node::ProcessingInstruction { target, data } => {
                self.writer.write_all(b"<?")?;
                self.writer.write_all(target.as_bytes())?;
                if !data.is_empty() {
                    self.writer.write_all(b" ")?;
                    self.writer.write_all(data.as_bytes())?;
                }
                self.writer.write_all(b"?>")?;
                Ok(())
            },
            Node::DocType(doctype) => {
                self.writer.write_all(b"<!DOCTYPE ")?;
                self.writer.write_all(doctype.as_bytes())?;
                self.writer.write_all(b">")?;
                Ok(())
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Element;

    #[test]
    fn test_escape_attribute() {
        let mut output = String::new();
        escape_attribute_to("hello", &mut output);
        assert_eq!(output, "hello");

        output.clear();
        escape_attribute_to("hello & world", &mut output);
        assert_eq!(output, "hello &amp; world");

        output.clear();
        escape_attribute_to("\"quoted\"", &mut output);
        assert_eq!(output, "&quot;quoted&quot;");

        output.clear();
        escape_attribute_to("<tag>", &mut output);
        assert_eq!(output, "&lt;tag&gt;");
    }

    #[test]
    fn test_escape_text() {
        let mut output = String::new();
        escape_text_to("hello", &mut output);
        assert_eq!(output, "hello");

        output.clear();
        escape_text_to("hello & world", &mut output);
        assert_eq!(output, "hello &amp; world");

        output.clear();
        escape_text_to("<tag>", &mut output);
        assert_eq!(output, "&lt;tag&gt;");

        output.clear();
        escape_text_to("\"quoted\"", &mut output);
        assert_eq!(output, "&quot;quoted&quot;");

        output.clear();
        escape_text_to("it's", &mut output);
        assert_eq!(output, "it&apos;s");
    }

    #[test]
    fn test_stringify_simple() {
        let mut doc = Document::new();
        doc.root.set_attr("width", "100");
        doc.root.set_attr("height", "100");

        let result = stringify(&doc).unwrap();
        assert!(result.contains("<svg width=\"100\" height=\"100\"/>"));
    }

    #[test]
    fn test_stringify_with_children() {
        let mut doc = Document::new();
        let mut rect = Element::new("rect");
        rect.set_attr("x", "10");
        rect.set_attr("y", "10");
        doc.root.add_child(Node::Element(rect));

        let result = stringify(&doc).unwrap();
        assert!(result.contains("<svg><rect x=\"10\" y=\"10\"/></svg>"));
    }

    #[test]
    fn test_stringify_pretty() {
        let mut doc = Document::new();
        let mut g = Element::new("g");
        let mut rect = Element::new("rect");
        rect.set_attr("x", "10");
        g.add_child(Node::Element(rect));
        doc.root.add_child(Node::Element(g));

        let config = StringifyConfig::pretty();
        let result = stringify_with_config(&doc, &config).unwrap();

        assert!(result.contains("\n"));
        assert!(result.contains("  <g>"));
        assert!(result.contains("    <rect"));
    }

    #[test]
    fn test_escape_performance() {
        // Test that fast path works
        let no_escape = "simple text without special chars";
        let mut output = String::new();
        escape_text_to(no_escape, &mut output);
        assert_eq!(output, no_escape);

        // Test escaping
        output.clear();
        escape_text_to("text & <tag>", &mut output);
        assert_eq!(output, "text &amp; &lt;tag&gt;");
    }

    #[test]
    fn test_size_estimation() {
        let mut element = Element::new("rect");
        element.set_attr("x", "10");
        element.set_attr("y", "20");
        element.add_child(Node::Text("content".into()));

        let estimated = estimate_element_size(&element);
        assert!(estimated > 20); // Should be reasonable estimate
    }

    #[test]
    fn test_namespace_output() {
        let mut doc = Document::new();
        doc.root.namespaces.insert("".into(), "http://www.w3.org/2000/svg".into());
        doc.root.namespaces.insert("xlink".into(), "http://www.w3.org/1999/xlink".into());

        let result = stringify(&doc).unwrap();
        assert!(result.contains("xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(result.contains("xmlns:xlink=\"http://www.w3.org/1999/xlink\""));
    }

    #[test]
    fn test_streaming_stringifier_matches_stringify_minified() {
        let mut doc = Document::new();
        doc.metadata.version = Some("1.0".into());
        doc.metadata.encoding = Some("UTF-8".into());
        doc.metadata.standalone = Some("no".into());
        doc.root.set_attr("width", "100");
        doc.root.set_attr("height", "100");
        doc.root.namespaces.insert("".into(), "http://www.w3.org/2000/svg".into());

        let mut g = Element::new("g");
        let mut rect = Element::new("rect");
        rect.set_attr("x", "10");
        rect.set_attr("y", "10");
        rect.set_attr("fill", "red");
        g.add_child(Node::Element(rect));
        g.add_child(Node::Comment("note".into()));
        doc.root.add_child(Node::Element(g));
        doc.root.add_child(Node::Text("it's \"ok\" & <safe>".into()));

        let expected = stringify(&doc).unwrap();

        let mut out = Vec::<u8>::new();
        let mut streaming = StreamingStringifier::new(&mut out, StringifyConfig::default());
        streaming.stringify(&doc).unwrap();
        let actual = String::from_utf8(out).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_streaming_stringifier_matches_stringify_pretty() {
        let mut doc = Document::new();
        let mut g = Element::new("g");
        let mut circle = Element::new("circle");
        circle.set_attr("cx", "5");
        circle.set_attr("cy", "6");
        g.add_child(Node::Element(circle));
        doc.root.add_child(Node::Element(g));

        let config = StringifyConfig::pretty();
        let expected = stringify_with_config(&doc, &config).unwrap();

        let mut out = Vec::<u8>::new();
        let mut streaming = StreamingStringifier::new(&mut out, config);
        streaming.stringify(&doc).unwrap();
        let actual = String::from_utf8(out).unwrap();

        assert_eq!(actual, expected);
    }
}
