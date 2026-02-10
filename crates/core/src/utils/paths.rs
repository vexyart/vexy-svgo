// this_file: crates/core/src/utils/paths.rs

//! Path data utilities for SVG processing
//!
//! Common functions for working with SVG path data across multiple plugins.

use std::fmt;

use crate::utils::numbers::NumberUtils;

/// Path data utilities
pub struct PathUtils;

impl PathUtils {
    /// Parse path data string into individual commands
    pub fn parse_path_data(data: &str) -> Vec<PathCommand> {
        let mut commands = Vec::new();
        let bytes = data.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            // Skip whitespace and commas
            while i < len && (bytes[i].is_ascii_whitespace() || bytes[i] == b',') {
                i += 1;
            }
            if i >= len {
                break;
            }

            let char_byte = bytes[i];
            // SVG commands are alphabetic. Note: we don't support scientific notation 'e' in scanning
            // currently, matching legacy behavior. If 'e' appears, it's treated as a command or
            // garbage.
            if char_byte.is_ascii_alphabetic() {
                let cmd = char_byte as char;
                i += 1;

                let mut params = Vec::new();

                loop {
                    // Skip whitespace/commas between params
                    while i < len && (bytes[i].is_ascii_whitespace() || bytes[i] == b',') {
                        i += 1;
                    }
                    if i >= len {
                        break;
                    }

                    // If next char is a command (alphabetic), stop param parsing
                    if bytes[i].is_ascii_alphabetic() {
                        break;
                    }

                    // Parse number
                    let start = i;
                    let mut has_decimal = false;

                    // Optional sign at start
                    if bytes[i] == b'+' || bytes[i] == b'-' {
                        i += 1;
                    }

                    // Digits and dot
                    while i < len {
                        let b = bytes[i];
                        if b.is_ascii_digit() {
                            i += 1;
                        } else if b == b'.' && !has_decimal {
                            has_decimal = true;
                            i += 1;
                        } else {
                            // Separator or next command or second dot/sign
                            break;
                        }
                    }

                    if i > start {
                        // We found a potential number token
                        // SAFETY: We only advanced over ASCII chars
                        let s = unsafe { std::str::from_utf8_unchecked(&bytes[start..i]) };
                        if let Ok(num) = s.parse::<f64>() {
                            params.push(num);
                        }
                    } else {
                        // Should not happen if we check is_ascii_alphabetic above,
                        // but prevents infinite loop if garbage encountered
                        i += 1;
                    }
                }
                commands.push(PathCommand { command: cmd, params });
            } else {
                // Unexpected character, skip
                i += 1;
            }
        }

        commands
    }

    /// Convert path commands back to string
    pub fn stringify_path_data(commands: &[PathCommand]) -> String {
        let mut result = String::new();

        for cmd in commands {
            result.push(cmd.command);

            for (i, &param) in cmd.params.iter().enumerate() {
                if i > 0 {
                    result.push(',');
                }
                result.push_str(&NumberUtils::format_number_minimal(param));
            }
        }

        result
    }

    /// Check if a path command is a move command
    pub fn is_move_command(cmd: char) -> bool {
        matches!(cmd, 'M' | 'm')
    }

    /// Check if a path command is a line command
    pub fn is_line_command(cmd: char) -> bool {
        matches!(cmd, 'L' | 'l' | 'H' | 'h' | 'V' | 'v')
    }

    /// Check if a path command is a curve command
    pub fn is_curve_command(cmd: char) -> bool {
        matches!(cmd, 'C' | 'c' | 'S' | 's' | 'Q' | 'q' | 'T' | 't')
    }

    /// Check if a path command is an arc command
    pub fn is_arc_command(cmd: char) -> bool {
        matches!(cmd, 'A' | 'a')
    }

    /// Check if a path command is a close command
    pub fn is_close_command(cmd: char) -> bool {
        matches!(cmd, 'Z' | 'z')
    }

    /// Convert relative command to absolute
    pub fn to_absolute_command(cmd: char) -> char {
        match cmd {
            'm' => 'M',
            'l' => 'L',
            'h' => 'H',
            'v' => 'V',
            'c' => 'C',
            's' => 'S',
            'q' => 'Q',
            't' => 'T',
            'a' => 'A',
            'z' => 'Z',
            _ => cmd, // Already absolute or unknown
        }
    }

    /// Convert absolute command to relative
    pub fn to_relative_command(cmd: char) -> char {
        match cmd {
            'M' => 'm',
            'L' => 'l',
            'H' => 'h',
            'V' => 'v',
            'C' => 'c',
            'S' => 's',
            'Q' => 'q',
            'T' => 't',
            'A' => 'a',
            'Z' => 'z',
            _ => cmd, // Already relative or unknown
        }
    }

    /// Check if a command is relative
    pub fn is_relative_command(cmd: char) -> bool {
        cmd.is_lowercase()
    }

    /// Get the number of parameters expected for a command
    pub fn get_param_count(cmd: char) -> usize {
        match cmd.to_ascii_uppercase() {
            'M' | 'L' => 2,
            'H' | 'V' => 1,
            'C' => 6,
            'S' | 'Q' => 4,
            'T' => 2,
            'A' => 7,
            'Z' => 0,
            _ => 0,
        }
    }
}

/// Represents a single path command
#[derive(Debug, Clone, PartialEq)]
pub struct PathCommand {
    pub command: char,
    pub params:  Vec<f64>,
}

impl PathCommand {
    /// Create a new path command
    pub fn new(command: char, params: Vec<f64>) -> Self {
        Self { command, params }
    }

    /// Check if this command has the expected number of parameters
    pub fn is_valid(&self) -> bool {
        let expected = PathUtils::get_param_count(self.command);
        expected == 0 || self.params.len() % expected == 0
    }
}

impl fmt::Display for PathCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.command)?;

        for (i, &param) in self.params.iter().enumerate() {
            if i > 0 {
                write!(f, ",")?;
            }
            write!(f, "{}", NumberUtils::format_number_minimal(param))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_path_data() {
        let commands = PathUtils::parse_path_data("M10,20 L30,40");
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].command, 'M');
        assert_eq!(commands[0].params, vec![10.0, 20.0]);
        assert_eq!(commands[1].command, 'L');
        assert_eq!(commands[1].params, vec![30.0, 40.0]);
    }

    #[test]
    fn test_stringify_path_data() {
        let commands = vec![
            PathCommand::new('M', vec![10.0, 20.0]),
            PathCommand::new('L', vec![30.0, 40.0]),
        ];
        let result = PathUtils::stringify_path_data(&commands);
        assert_eq!(result, "M10,20L30,40");
    }

    #[test]
    fn test_command_types() {
        assert!(PathUtils::is_move_command('M'));
        assert!(PathUtils::is_move_command('m'));
        assert!(PathUtils::is_line_command('L'));
        assert!(PathUtils::is_line_command('l'));
        assert!(PathUtils::is_curve_command('C'));
        assert!(PathUtils::is_curve_command('c'));
        assert!(PathUtils::is_arc_command('A'));
        assert!(PathUtils::is_arc_command('a'));
        assert!(PathUtils::is_close_command('Z'));
        assert!(PathUtils::is_close_command('z'));
    }

    #[test]
    fn test_command_conversion() {
        assert_eq!(PathUtils::to_absolute_command('m'), 'M');
        assert_eq!(PathUtils::to_absolute_command('M'), 'M');
        assert_eq!(PathUtils::to_relative_command('M'), 'm');
        assert_eq!(PathUtils::to_relative_command('m'), 'm');
        assert!(PathUtils::is_relative_command('m'));
        assert!(!PathUtils::is_relative_command('M'));
    }

    #[test]
    fn test_param_count() {
        assert_eq!(PathUtils::get_param_count('M'), 2);
        assert_eq!(PathUtils::get_param_count('L'), 2);
        assert_eq!(PathUtils::get_param_count('C'), 6);
        assert_eq!(PathUtils::get_param_count('Z'), 0);
    }

    #[test]
    fn test_path_command_validity() {
        let cmd = PathCommand::new('M', vec![10.0, 20.0]);
        assert!(cmd.is_valid());

        let invalid_cmd = PathCommand::new('M', vec![10.0]);
        assert!(!invalid_cmd.is_valid());
    }

    #[test]
    fn test_parse_compact_svg_notation() {
        let commands = PathUtils::parse_path_data("c6.625.191 11.52.01 11.828-2.044");
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].command, 'c');
        assert_eq!(commands[0].params.len(), 6);
        assert!((commands[0].params[0] - 6.625).abs() < 0.001);
        assert!((commands[0].params[1] - 0.191).abs() < 0.001);
        assert!((commands[0].params[2] - 11.52).abs() < 0.001);
        assert!((commands[0].params[3] - 0.01).abs() < 0.001);
        assert!((commands[0].params[4] - 11.828).abs() < 0.001);
        assert!((commands[0].params[5] - (-2.044)).abs() < 0.001);
    }
}
