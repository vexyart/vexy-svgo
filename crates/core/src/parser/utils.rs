// this_file: crates/core/src/parser/utils.rs

use crate::parser::error::{DetailedParseError, ErrorCategory, ErrorContext, ErrorSeverity, ParseError};

/// Calculate line and column from byte position
pub fn calculate_line_and_column(input: &str, byte_pos: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;

    for (i, ch) in input.char_indices() {
        if i >= byte_pos {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line, col)
}

/// Create a detailed parse error with full context
pub fn create_detailed_error_with_category(
    file_path: &Option<String>,
    input: &str,
    byte_pos: usize,
    message: String,
    severity: ErrorSeverity,
    category: ErrorCategory,
    suggestions: Vec<String>,
) -> ParseError {
    let (line, column) = calculate_line_and_column(input, byte_pos);

    // Extract context lines
    let lines: Vec<&str> = input.lines().collect();
    let mut context_lines = Vec::new();
    let mut error_line_index = 0;

    // Get 2 lines before and after the error
    let start_line = line.saturating_sub(3);
    let end_line = (line + 2).min(lines.len());

    for i in start_line..end_line {
        if let Some(line_content) = lines.get(i) {
            context_lines.push(line_content.to_string());
            if i + 1 == line {
                error_line_index = context_lines.len() - 1;
            }
        }
    }

    let context = if !context_lines.is_empty() {
        Some(ErrorContext {
            lines: context_lines,
            error_line_index,
            error_column: column.saturating_sub(1),
        })
    } else {
        None
    };

    ParseError::DetailedError(Box::new(DetailedParseError {
        file_path: file_path.clone(),
        line,
        column,
        byte_offset: byte_pos,
        message,
        severity,
        category,
        context,
        suggestions,
    }))
}
