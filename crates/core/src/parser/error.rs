// this_file: crates/core/src/parser/error.rs

use std::fmt;

use thiserror::Error;

/// Parse error types
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("XML parsing error: {0}")]
    XmlError(#[from] quick_xml::Error),

    #[error("Attribute parsing error: {0}")]
    AttrError(String),

    #[error("Invalid UTF-8: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("Document structure error: {0}")]
    StructureError(String),

    #[error("Unexpected end of document")]
    UnexpectedEnd,

    #[error("Invalid entity reference: {0}")]
    EntityError(String),

    #[error("Invalid namespace declaration: {0}")]
    NamespaceError(String),

    #[error("Malformed DOCTYPE: {0}")]
    DoctypeError(String),

    #[error("Unsupported XML feature: {0}")]
    UnsupportedFeature(String),

    #[error("Security violation: {0}")]
    SecurityError(String),

    #[error("File I/O error: {0}")]
    FileIoError(#[from] std::io::Error),

    #[error("{0}")]
    DetailedError(Box<DetailedParseError>),
}

/// Detailed parse error with context
#[derive(Debug, Clone)]
pub struct DetailedParseError {
    /// File path (if available)
    pub file_path:   Option<String>,
    /// Line number (1-based)
    pub line:        usize,
    /// Column number (1-based)
    pub column:      usize,
    /// Byte offset in the source
    pub byte_offset: usize,
    /// Error message
    pub message:     String,
    /// Error severity level
    pub severity:    ErrorSeverity,
    /// Error category for better tooling integration
    pub category:    ErrorCategory,
    /// Source code context
    pub context:     Option<ErrorContext>,
    /// Suggested fixes (if any)
    pub suggestions: Vec<String>,
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Error,
    Warning,
    Info,
}

/// Error categories for better categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Syntax,
    Structure,
    Namespace,
    Entity,
    Encoding,
    Security,
    Performance,
}

/// Error context with source code snippet
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Lines of source code around the error
    pub lines:            Vec<String>,
    /// Index of the error line in the lines vector
    pub error_line_index: usize,
    /// Column position in the error line
    pub error_column:     usize,
}

impl fmt::Display for DetailedParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format: severity: file.svg:line:column: category: error message
        write!(f, "{:?}: ", self.severity)?;

        if let Some(ref path) = self.file_path {
            write!(f, "{path}:")?;
        }
        write!(f, "{}:{}: {:?}: {}", self.line, self.column, self.category, self.message)?;

        // Add source context if available
        if let Some(ref ctx) = self.context {
            writeln!(f)?;
            writeln!(f)?;

            // Display lines with line numbers
            let start_line = self.line.saturating_sub(ctx.error_line_index);
            for (i, line) in ctx.lines.iter().enumerate() {
                let line_num = start_line + i;
                let prefix = if i == ctx.error_line_index { ">" } else { " " };
                writeln!(f, "{prefix} {line_num:3} | {line}")?;

                // Add error pointer on the error line
                if i == ctx.error_line_index {
                    let spaces = " ".repeat(ctx.error_column + 6);
                    writeln!(f, "{spaces} ^")?;
                }
            }
        }

        // Add suggestions if available
        if !self.suggestions.is_empty() {
            writeln!(f)?;
            writeln!(f, "Suggestions:")?;
            for suggestion in &self.suggestions {
                writeln!(f, "  - {suggestion}")?;
            }
        }

        Ok(())
    }
}

/// Parse result type
pub type ParseResult<T> = Result<T, ParseError>;
