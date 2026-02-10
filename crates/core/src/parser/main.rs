// this_file: crates/core/src/parser/main.rs

//! SVG parser using quick-xml
//!
//! This module provides functionality to parse SVG strings into our custom AST
//! using the quick-xml crate for fast streaming XML parsing.
//!
//! Features streaming parsing optimizations for large documents,
//! efficient buffer management, and memory-conscious parsing strategies.

use std::io::{BufRead, BufReader};

use crate::ast::Document;
use crate::error::VexyError;
use crate::parser::config::StreamingConfig;
use crate::parser::streaming::StreamingParser;

/// SVG parser with streaming optimizations
pub struct Parser {
    /// Whether to preserve whitespace
    preserve_whitespace: bool,
    /// Whether to preserve comments
    preserve_comments:   bool,
    /// Whether to expand XML entities
    expand_entities:     bool,

    /// Streaming configuration
    streaming_config: StreamingConfig,
}

impl Parser {
    /// Create a new parser with default settings
    pub fn new() -> Self {
        Self {
            preserve_whitespace: false,
            preserve_comments:   true,
            expand_entities:     true,
            streaming_config:    StreamingConfig::default(),
        }
    }

    /// Create a new parser with streaming configuration
    pub fn with_streaming_config(config: StreamingConfig) -> Self {
        Self {
            preserve_whitespace: false,
            preserve_comments:   true,
            expand_entities:     true,
            streaming_config:    config,
        }
    }

    /// Set whether to preserve whitespace
    pub fn preserve_whitespace(mut self, preserve: bool) -> Self {
        self.preserve_whitespace = preserve;
        self
    }

    /// Set whether to preserve comments
    pub fn preserve_comments(mut self, preserve: bool) -> Self {
        self.preserve_comments = preserve;
        self
    }

    /// Set whether to expand XML entities
    pub fn expand_entities(mut self, expand: bool) -> Self {
        self.expand_entities = expand;
        self
    }

    /// Set streaming configuration
    pub fn streaming_config(mut self, config: StreamingConfig) -> Self {
        self.streaming_config = config;
        self
    }

    /// Parse an SVG string into a Document (static method for convenience)
    pub fn parse_svg_string(input: &str) -> Result<Document<'static>, VexyError> {
        let parser = Self::new();
        parser.parse(input)
    }

    /// Parse an SVG string into a Document
    /// For large documents, consider using `parse_streaming` for better memory efficiency
    pub fn parse(&self, input: &str) -> Result<Document<'static>, VexyError> {
        // For very large inputs, automatically use streaming parser
        if input.len() > self.streaming_config.buffer_size * 4 {
            return self.parse_streaming_from_str(input);
        }

        self.parse_internal(input)
    }

    /// Internal parsing implementation
    fn parse_internal(&self, input: &str) -> Result<Document<'static>, VexyError> {
        let mut streaming_parser = StreamingParser::new(
            BufReader::new(input.as_bytes()),
            self.streaming_config.clone(),
            self.preserve_whitespace,
            self.preserve_comments,
            self.expand_entities,
            None,
        );
        streaming_parser.parse()
    }

    /// Parse from a streaming source for memory efficiency with large files
    pub fn parse_streaming<R: BufRead>(&self, reader: R) -> Result<Document<'static>, VexyError> {
        let mut streaming_parser = StreamingParser::new(
            reader,
            self.streaming_config.clone(),
            self.preserve_whitespace,
            self.preserve_comments,
            self.expand_entities,
            None,
        );
        streaming_parser.parse()
    }

    /// Parse from a large string using streaming approach
    fn parse_streaming_from_str(&self, input: &str) -> Result<Document<'static>, VexyError> {
        let cursor = std::io::Cursor::new(input);
        let buf_reader = BufReader::with_capacity(self.streaming_config.buffer_size, cursor);
        self.parse_streaming(buf_reader)
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to parse an SVG string
pub fn parse_svg(input: &str) -> Result<Document<'static>, VexyError> {
    Parser::new().parse(input)
}

/// Convenience function to parse a large SVG with streaming
pub fn parse_svg_streaming<R: BufRead>(reader: R) -> Result<Document<'static>, VexyError> {
    Parser::new().parse_streaming(reader)
}

/// Parse SVG from a file with automatic streaming for large files
pub fn parse_svg_file<P: AsRef<std::path::Path>>(path: P) -> Result<Document<'static>, VexyError> {
    let file = std::fs::File::open(&path)?;
    let metadata = file.metadata()?;

    let parser = Parser::new();

    // Use streaming for files larger than 1MB
    if metadata.len() > 1024 * 1024 {
        let buf_reader = BufReader::with_capacity(128 * 1024, file);
        parser.parse_streaming(buf_reader)
    } else {
        let content = std::fs::read_to_string(path)?;
        parser.parse(&content)
    }
}
