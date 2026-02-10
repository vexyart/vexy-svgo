// this_file: crates/plugin-sdk/src/plugins/convert_path_data/mod.rs

//! Convert path data to relative or absolute, optimize segments, simplify curves
//!
//! This plugin optimizes path data by:
//! - Converting between absolute and relative commands
//! - Removing redundant commands
//! - Optimizing number precision
//! - Simplifying curves where possible

pub mod abs_to_rel;
pub mod rel_to_abs;
pub mod normalize;

use crate::Plugin;
use anyhow::Result;
use lyon::{
    geom::{Point, Vector, CubicBezierSegment, QuadraticBezierSegment},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vexy_svgo_core::ast::{Document, Element, Node};

/// Default decimal precision for path coordinates
const DEFAULT_FLOAT_PRECISION: u8 = 3;

/// Default decimal precision for transform values
const DEFAULT_TRANSFORM_PRECISION: u8 = 5;

/// Configuration for the convertPathData plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConvertPathDataConfig {
    #[serde(default = "default_float_precision")]
    pub float_precision: u8,

    #[serde(default = "default_transform_precision")]
    pub transform_precision: u8,

    #[serde(default = "default_true")]
    pub remove_useless: bool,

    #[serde(default = "default_true")]
    pub collapse_repeated: bool,

    #[serde(default = "default_true")]
    pub utilize_absolute: bool,

    #[serde(default = "default_true")]
    pub leading_zero: bool,

    #[serde(default = "default_true")]
    pub negative_extra_space: bool,

    /// Convert curves to arcs where geometrically appropriate
    #[serde(default)]
    pub make_arcs: MakeArcsConfig,

    /// Straighten curves that are nearly straight lines
    #[serde(default = "default_false")]
    pub straight_curves: bool,

    /// Convert cubic bezier curves to quadratic where possible
    #[serde(default = "default_false")]
    pub convert_to_q: bool,

    /// Tolerance for curve straightening (smaller = more strict)
    #[serde(default = "default_curve_tolerance")]
    pub curve_tolerance: f64,

    /// Tolerance for arc detection (smaller = more strict)
    #[serde(default = "default_arc_tolerance")]
    pub arc_tolerance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MakeArcsConfig {
    Enabled(bool),
    Params(MakeArcsParams),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MakeArcsParams {
    #[serde(default = "default_make_arcs_threshold")]
    pub threshold: f64,
    #[serde(default = "default_make_arcs_tolerance")]
    pub tolerance: f64,
}

impl MakeArcsConfig {
    fn is_enabled(&self) -> bool {
        match self {
            Self::Enabled(enabled) => *enabled,
            Self::Params(_) => true,
        }
    }

    fn tolerance(&self, default_tolerance: f64) -> f64 {
        match self {
            Self::Params(params) => params.tolerance,
            Self::Enabled(_) => default_tolerance,
        }
    }
}

impl Default for MakeArcsConfig {
    fn default() -> Self {
        Self::Enabled(false)
    }
}

fn default_float_precision() -> u8 {
    DEFAULT_FLOAT_PRECISION
}

fn default_transform_precision() -> u8 {
    DEFAULT_TRANSFORM_PRECISION
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_curve_tolerance() -> f64 {
    0.1
}

fn default_arc_tolerance() -> f64 {
    0.5
}

fn default_make_arcs_threshold() -> f64 {
    2.5
}

fn default_make_arcs_tolerance() -> f64 {
    0.5
}

impl Default for ConvertPathDataConfig {
    fn default() -> Self {
        Self {
            float_precision: default_float_precision(),
            transform_precision: default_transform_precision(),
            remove_useless: true,
            collapse_repeated: true,
            utilize_absolute: true,
            leading_zero: true,
            negative_extra_space: true,
            make_arcs: MakeArcsConfig::default(),
            straight_curves: false,
            convert_to_q: false,
            curve_tolerance: default_curve_tolerance(),
            arc_tolerance: default_arc_tolerance(),
        }
    }
}

/// Plugin for optimizing path data
pub struct ConvertPathDataPlugin {
    config: ConvertPathDataConfig,
}

impl ConvertPathDataPlugin {
    pub fn new() -> Self {
        Self {
            config: ConvertPathDataConfig::default(),
        }
    }

    pub fn with_config(config: ConvertPathDataConfig) -> Self {
        Self { config }
    }

    fn parse_config(params: &Value) -> Result<ConvertPathDataConfig> {
        if params.is_null() {
            Ok(ConvertPathDataConfig::default())
        } else {
            serde_json::from_value(params.clone())
                .map_err(|e| anyhow::anyhow!("Invalid plugin configuration: {}", e))
        }
    }

    /// Recursively optimize paths in an element and its children
    fn optimize_paths_in_element(&self, element: &mut Element) {
        // Process this element if it's a path
        if element.name == "path" {
            if let Some(d) = element.attr("d") {
                match optimize_path_data(d, &self.config) {
                    Ok(optimized) => {
                        element.set_attr("d", &optimized);
                    }
                    Err(e) => {
                        // Log error but continue processing other paths
                        eprintln!("Warning: Failed to optimize path data: {}", e);
                    }
                }
            }
        }

        // Process children
        let mut i = 0;
        while i < element.children.len() {
            if let Node::Element(child) = &mut element.children[i] {
                self.optimize_paths_in_element(child);
            }
            i += 1;
        }
    }
}

impl Default for ConvertPathDataPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ConvertPathDataPlugin {
    fn name(&self) -> &'static str {
        "convertPathData"
    }

    fn description(&self) -> &'static str {
        "converts path data to relative or absolute, optimizes segments, simplifies curves"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        Self::parse_config(params)?;
        Ok(())
    }

    fn configure(&mut self, params: &Value) -> Result<()> {
        self.config = Self::parse_config(params)?;
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> Result<()> {
        self.optimize_paths_in_element(&mut document.root);
        Ok(())
    }
}

/// Path command types
#[derive(Debug, Clone, Copy, PartialEq)]
enum CommandType {
    MoveTo,
    LineTo,
    HorizontalLineTo,
    VerticalLineTo,
    CurveTo,
    SmoothCurveTo,
    QuadraticBezier,
    SmoothQuadraticBezier,
    Arc,
    ClosePath,
}

/// Represents a path command with its parameters
#[derive(Debug, Clone)]
struct PathCommand {
    cmd_type: CommandType,
    is_absolute: bool,
    params: Vec<f64>,
}

impl PathCommand {
    /// Get the command character
    fn get_char(&self) -> char {
        match (self.cmd_type, self.is_absolute) {
            (CommandType::MoveTo, true) => 'M',
            (CommandType::MoveTo, false) => 'm',
            (CommandType::LineTo, true) => 'L',
            (CommandType::LineTo, false) => 'l',
            (CommandType::HorizontalLineTo, true) => 'H',
            (CommandType::HorizontalLineTo, false) => 'h',
            (CommandType::VerticalLineTo, true) => 'V',
            (CommandType::VerticalLineTo, false) => 'v',
            (CommandType::CurveTo, true) => 'C',
            (CommandType::CurveTo, false) => 'c',
            (CommandType::SmoothCurveTo, true) => 'S',
            (CommandType::SmoothCurveTo, false) => 's',
            (CommandType::QuadraticBezier, true) => 'Q',
            (CommandType::QuadraticBezier, false) => 'q',
            (CommandType::SmoothQuadraticBezier, true) => 'T',
            (CommandType::SmoothQuadraticBezier, false) => 't',
            (CommandType::Arc, true) => 'A',
            (CommandType::Arc, false) => 'a',
            (CommandType::ClosePath, _) => 'z',
        }
    }
}

/// Parse a path data string into commands
fn parse_path_data(path_data: &str) -> Result<Vec<PathCommand>> {
    let mut commands = Vec::new();
    let mut chars = path_data.chars().peekable();
    let mut current_nums = Vec::new();
    let mut current_num = String::new();
    let mut last_cmd_type = None;
    let mut in_number = false;

    for ch in chars.by_ref() {
        match ch {
            'M' | 'm' | 'L' | 'l' | 'H' | 'h' | 'V' | 'v' | 'C' | 'c' | 'S' | 's' | 'Q' | 'q'
            | 'T' | 't' | 'A' | 'a' | 'Z' | 'z' => {
                // Finish current number if any
                if !current_num.is_empty() {
                    if let Ok(num) = current_num.parse::<f64>() {
                        current_nums.push(num);
                    }
                    current_num.clear();
                    in_number = false;
                }

                // Process accumulated numbers with previous command
                if let Some(cmd_type) = last_cmd_type {
                    process_accumulated_params(&mut commands, cmd_type, &mut current_nums)?;
                }

                // Parse new command
                let (cmd_type, is_absolute) = parse_command_char(ch)?;

                if cmd_type == CommandType::ClosePath {
                    commands.push(PathCommand {
                        cmd_type,
                        is_absolute: true,
                        params: vec![],
                    });
                    last_cmd_type = None;
                } else {
                    last_cmd_type = Some((cmd_type, is_absolute));
                }
            }
            '0'..='9' | '.' | '-' | '+' | 'e' | 'E' => {
                if ch == '-' || ch == '+' {
                    // Start new number if not at beginning of current number
                    if !current_num.is_empty() && in_number {
                        if let Ok(num) = current_num.parse::<f64>() {
                            current_nums.push(num);
                        }
                        current_num.clear();
                    }
                }
                current_num.push(ch);
                in_number = true;
            }
            ' ' | ',' | '\t' | '\n' | '\r' => {
                // Number separator
                if !current_num.is_empty() {
                    if let Ok(num) = current_num.parse::<f64>() {
                        current_nums.push(num);
                    }
                    current_num.clear();
                    in_number = false;
                }
            }
            _ => {
                // Ignore other characters
            }
        }
    }

    // Finish last number
    if !current_num.is_empty() {
        if let Ok(num) = current_num.parse::<f64>() {
            current_nums.push(num);
        }
    }

    // Process final accumulated numbers
    if let Some(cmd_type) = last_cmd_type {
        process_accumulated_params(&mut commands, cmd_type, &mut current_nums)?;
    }

    Ok(commands)
}

/// Parse a command character into its type and whether it's absolute
fn parse_command_char(ch: char) -> Result<(CommandType, bool)> {
    match ch {
        'M' => Ok((CommandType::MoveTo, true)),
        'm' => Ok((CommandType::MoveTo, false)),
        'L' => Ok((CommandType::LineTo, true)),
        'l' => Ok((CommandType::LineTo, false)),
        'H' => Ok((CommandType::HorizontalLineTo, true)),
        'h' => Ok((CommandType::HorizontalLineTo, false)),
        'V' => Ok((CommandType::VerticalLineTo, true)),
        'v' => Ok((CommandType::VerticalLineTo, false)),
        'C' => Ok((CommandType::CurveTo, true)),
        'c' => Ok((CommandType::CurveTo, false)),
        'S' => Ok((CommandType::SmoothCurveTo, true)),
        's' => Ok((CommandType::SmoothCurveTo, false)),
        'Q' => Ok((CommandType::QuadraticBezier, true)),
        'q' => Ok((CommandType::QuadraticBezier, false)),
        'T' => Ok((CommandType::SmoothQuadraticBezier, true)),
        't' => Ok((CommandType::SmoothQuadraticBezier, false)),
        'A' => Ok((CommandType::Arc, true)),
        'a' => Ok((CommandType::Arc, false)),
        'Z' | 'z' => Ok((CommandType::ClosePath, true)),
        _ => Err(anyhow::anyhow!("Unknown command character: {}", ch)),
    }
}

/// Process accumulated parameters for a command type
fn process_accumulated_params(
    commands: &mut Vec<PathCommand>,
    (cmd_type, is_absolute): (CommandType, bool),
    params: &mut Vec<f64>,
) -> Result<()> {
    let expected = match cmd_type {
        CommandType::MoveTo | CommandType::LineTo => 2,
        CommandType::HorizontalLineTo | CommandType::VerticalLineTo => 1,
        CommandType::CurveTo => 6,
        CommandType::SmoothCurveTo | CommandType::QuadraticBezier => 4,
        CommandType::SmoothQuadraticBezier => 2,
        CommandType::Arc => 7,
        CommandType::ClosePath => 0,
    };

    if expected == 0 {
        return Ok(());
    }

    // Process params in chunks
    while params.len() >= expected {
        let chunk: Vec<f64> = params.drain(..expected).collect();

        // Special case: MoveTo followed by implicit LineTo
        let actual_cmd_type = if cmd_type == CommandType::MoveTo && !commands.is_empty() {
            CommandType::LineTo
        } else {
            cmd_type
        };

        commands.push(PathCommand {
            cmd_type: actual_cmd_type,
            is_absolute,
            params: chunk,
        });
    }

    if !params.is_empty() {
        // Leftover params - this is technically an error but we'll ignore them
        params.clear();
    }

    Ok(())
}

/// Optimize path data string
fn optimize_path_data(path_data: &str, config: &ConvertPathDataConfig) -> Result<String> {
    // Parse path data
    let mut commands = parse_path_data(path_data)?;

    // Track current position for relative/absolute conversions
    let mut current_x = 0.0;
    let mut current_y = 0.0;
    let mut start_x = 0.0;
    let mut start_y = 0.0;

    // Convert to absolute coordinates for processing
    for cmd in &mut commands {
        match cmd.cmd_type {
            CommandType::MoveTo => {
                if !cmd.is_absolute && cmd.params.len() >= 2 {
                    cmd.params[0] += current_x;
                    cmd.params[1] += current_y;
                    cmd.is_absolute = true;
                }
                if cmd.params.len() >= 2 {
                    current_x = cmd.params[0];
                    current_y = cmd.params[1];
                    start_x = current_x;
                    start_y = current_y;
                }
            }
            CommandType::LineTo => {
                if !cmd.is_absolute && cmd.params.len() >= 2 {
                    cmd.params[0] += current_x;
                    cmd.params[1] += current_y;
                    cmd.is_absolute = true;
                }
                if cmd.params.len() >= 2 {
                    current_x = cmd.params[0];
                    current_y = cmd.params[1];
                }
            }
            CommandType::HorizontalLineTo => {
                if !cmd.is_absolute && !cmd.params.is_empty() {
                    cmd.params[0] += current_x;
                    cmd.is_absolute = true;
                }
                if !cmd.params.is_empty() {
                    current_x = cmd.params[0];
                }
            }
            CommandType::VerticalLineTo => {
                if !cmd.is_absolute && !cmd.params.is_empty() {
                    cmd.params[0] += current_y;
                    cmd.is_absolute = true;
                }
                if !cmd.params.is_empty() {
                    current_y = cmd.params[0];
                }
            }
            CommandType::CurveTo => {
                if !cmd.is_absolute && cmd.params.len() >= 6 {
                    cmd.params[0] += current_x;
                    cmd.params[1] += current_y;
                    cmd.params[2] += current_x;
                    cmd.params[3] += current_y;
                    cmd.params[4] += current_x;
                    cmd.params[5] += current_y;
                    cmd.is_absolute = true;
                }
                if cmd.params.len() >= 6 {
                    current_x = cmd.params[4];
                    current_y = cmd.params[5];
                }
            }
            CommandType::SmoothCurveTo => {
                if !cmd.is_absolute && cmd.params.len() >= 4 {
                    cmd.params[0] += current_x;
                    cmd.params[1] += current_y;
                    cmd.params[2] += current_x;
                    cmd.params[3] += current_y;
                    cmd.is_absolute = true;
                }
                if cmd.params.len() >= 4 {
                    current_x = cmd.params[2];
                    current_y = cmd.params[3];
                }
            }
            CommandType::QuadraticBezier => {
                if !cmd.is_absolute && cmd.params.len() >= 4 {
                    cmd.params[0] += current_x;
                    cmd.params[1] += current_y;
                    cmd.params[2] += current_x;
                    cmd.params[3] += current_y;
                    cmd.is_absolute = true;
                }
                if cmd.params.len() >= 4 {
                    current_x = cmd.params[2];
                    current_y = cmd.params[3];
                }
            }
            CommandType::SmoothQuadraticBezier => {
                if !cmd.is_absolute && cmd.params.len() >= 2 {
                    cmd.params[0] += current_x;
                    cmd.params[1] += current_y;
                    cmd.is_absolute = true;
                }
                if cmd.params.len() >= 2 {
                    current_x = cmd.params[0];
                    current_y = cmd.params[1];
                }
            }
            CommandType::Arc => {
                if !cmd.is_absolute && cmd.params.len() >= 7 {
                    cmd.params[5] += current_x;
                    cmd.params[6] += current_y;
                    cmd.is_absolute = true;
                }
                if cmd.params.len() >= 7 {
                    current_x = cmd.params[5];
                    current_y = cmd.params[6];
                }
            }
            CommandType::ClosePath => {
                current_x = start_x;
                current_y = start_y;
            }
        }
    }

    // Apply optimizations
    if config.remove_useless {
        commands = remove_useless_commands(commands);
    }

    if config.collapse_repeated {
        commands = collapse_repeated_commands(commands);
    }

    // Apply advanced geometric optimizations
    if config.straight_curves {
        commands = straighten_curves(commands, config.curve_tolerance);
    }

    if config.convert_to_q {
        commands = convert_cubic_to_quadratic(commands, config.curve_tolerance);
    }

    if config.make_arcs.is_enabled() {
        commands = convert_curves_to_arcs(commands, config.make_arcs.tolerance(config.arc_tolerance));
    }

    // Convert back to string
    stringify_commands(
        &commands,
        config.float_precision,
        config.utilize_absolute,
        config.leading_zero,
        config.negative_extra_space,
    )
}

/// Remove useless commands (e.g., LineTo to current position)
fn remove_useless_commands(mut commands: Vec<PathCommand>) -> Vec<PathCommand> {
    let mut result = Vec::new();
    let mut current_x = 0.0;
    let mut current_y = 0.0;

    for cmd in commands.drain(..) {
        let mut keep = true;

        match cmd.cmd_type {
            CommandType::LineTo => {
                if cmd.params.len() >= 2 {
                    // Remove LineTo that goes to current position
                    if (cmd.params[0] - current_x).abs() < f64::EPSILON
                        && (cmd.params[1] - current_y).abs() < f64::EPSILON
                    {
                        keep = false;
                    } else {
                        current_x = cmd.params[0];
                        current_y = cmd.params[1];
                    }
                }
            }
            CommandType::HorizontalLineTo => {
                if !cmd.params.is_empty() {
                    if (cmd.params[0] - current_x).abs() < f64::EPSILON {
                        keep = false;
                    } else {
                        current_x = cmd.params[0];
                    }
                }
            }
            CommandType::VerticalLineTo => {
                if !cmd.params.is_empty() {
                    if (cmd.params[0] - current_y).abs() < f64::EPSILON {
                        keep = false;
                    } else {
                        current_y = cmd.params[0];
                    }
                }
            }
            CommandType::MoveTo => {
                if cmd.params.len() >= 2 {
                    current_x = cmd.params[0];
                    current_y = cmd.params[1];
                }
            }
            CommandType::CurveTo => {
                if cmd.params.len() >= 6 {
                    current_x = cmd.params[4];
                    current_y = cmd.params[5];
                }
            }
            CommandType::SmoothCurveTo => {
                if cmd.params.len() >= 4 {
                    current_x = cmd.params[2];
                    current_y = cmd.params[3];
                }
            }
            CommandType::QuadraticBezier => {
                if cmd.params.len() >= 4 {
                    current_x = cmd.params[2];
                    current_y = cmd.params[3];
                }
            }
            CommandType::SmoothQuadraticBezier => {
                if cmd.params.len() >= 2 {
                    current_x = cmd.params[0];
                    current_y = cmd.params[1];
                }
            }
            CommandType::Arc => {
                if cmd.params.len() >= 7 {
                    current_x = cmd.params[5];
                    current_y = cmd.params[6];
                }
            }
            _ => {}
        }

        if keep {
            result.push(cmd);
        }
    }

    result
}

/// Collapse repeated commands where possible
fn collapse_repeated_commands(commands: Vec<PathCommand>) -> Vec<PathCommand> {
    if commands.is_empty() {
        return commands;
    }
    
    let mut result = Vec::new();
    let mut current_x = 0.0;
    let mut current_y = 0.0;
    let mut i = 0;
    
    while i < commands.len() {
        let cmd = commands[i].clone();
        
        // Check if this command can be collapsed with the previous one
        if i > 0 && can_collapse_commands(&result[result.len() - 1], &cmd) {
            let len = result.len();
            let prev_cmd = &mut result[len - 1];
            
            // Collapse the commands based on type
            match cmd.cmd_type {
                CommandType::MoveTo => {
                    // m 10 20 m 5 5 -> m 15 25
                    if cmd.params.len() >= 2 {
                        prev_cmd.params[0] += cmd.params[0];
                        prev_cmd.params[1] += cmd.params[1];
                    }
                }
                CommandType::LineTo => {
                    // Consecutive line commands can be collapsed into a single command
                    // l 10 20 l 5 5 -> l 15 25 (for relative)
                    if cmd.params.len() >= 2 && !cmd.is_absolute {
                        prev_cmd.params[0] += cmd.params[0];
                        prev_cmd.params[1] += cmd.params[1];
                    } else {
                        // For absolute commands, we can't simply add the values
                        result.push(cmd);
                    }
                }
                CommandType::HorizontalLineTo => {
                    // h 10 h 20 -> h 30 (if same sign)
                    if !cmd.params.is_empty() {
                        prev_cmd.params[0] += cmd.params[0];
                    }
                }
                CommandType::VerticalLineTo => {
                    // v 10 v 20 -> v 30 (if same sign)
                    if !cmd.params.is_empty() {
                        prev_cmd.params[0] += cmd.params[0];
                    }
                }
                _ => {
                    // For other commands, just add this one
                    result.push(cmd);
                }
            }
        } else {
            result.push(cmd);
        }
        
        // Update position tracking
        if let Some(last_cmd) = result.last() {
            update_position(last_cmd, &mut current_x, &mut current_y);
        }
        
        i += 1;
    }
    
    result
}

/// Check if two commands can be collapsed together
fn can_collapse_commands(prev: &PathCommand, current: &PathCommand) -> bool {
    // Commands must be of the same type
    if prev.cmd_type != current.cmd_type {
        return false;
    }
    
    // Commands must have same absolute/relative nature
    if prev.is_absolute != current.is_absolute {
        return false;
    }
    
    // Only collapse specific command types
    match current.cmd_type {
        CommandType::MoveTo => {
            // Can always collapse moveto commands
            true
        }
        CommandType::LineTo => {
            // Can collapse relative LineTo commands
            // Don't collapse absolute LineTo as they set specific positions
            !current.is_absolute
        }
        CommandType::HorizontalLineTo => {
            // Can collapse if both values have the same sign (both positive or both negative)
            if prev.params.is_empty() || current.params.is_empty() {
                return false;
            }
            (prev.params[0] >= 0.0) == (current.params[0] >= 0.0)
        }
        CommandType::VerticalLineTo => {
            // Can collapse if both values have the same sign (both positive or both negative)
            if prev.params.is_empty() || current.params.is_empty() {
                return false;
            }
            (prev.params[0] >= 0.0) == (current.params[0] >= 0.0)
        }
        _ => false, // Don't collapse other command types
    }
}

/// Convert commands back to string format
fn stringify_commands(
    commands: &[PathCommand],
    precision: u8,
    utilize_absolute: bool,
    leading_zero: bool,
    negative_extra_space: bool,
) -> Result<String> {
    let mut result = String::new();
    let mut last_cmd_char = '\0';
    let mut current_x = 0.0;
    let mut current_y = 0.0;

    for (i, cmd) in commands.iter().enumerate() {
        // Decide whether to use absolute or relative
        let mut use_absolute = cmd.is_absolute;
        if utilize_absolute && i > 0 {
            // Calculate which is shorter
            use_absolute = should_use_absolute(cmd, current_x, current_y, precision);
        }

        // Get the command character
        let cmd_char = if use_absolute {
            cmd.get_char().to_ascii_uppercase()
        } else {
            cmd.get_char().to_ascii_lowercase()
        };

        // Add command character if different from last
        if cmd_char != last_cmd_char || cmd.cmd_type == CommandType::MoveTo {
            if !result.is_empty() && cmd_char != 'Z' && cmd_char != 'z' {
                result.push(' ');
            }
            result.push(cmd_char);
            last_cmd_char = cmd_char;
        } else if !result.is_empty() {
            result.push(' ');
        }

        // Add parameters
        let params = if use_absolute {
            cmd.params.clone()
        } else {
            convert_to_relative(cmd, current_x, current_y)
        };

        for (j, &param) in params.iter().enumerate() {
            if j > 0 || (cmd_char != last_cmd_char && cmd_char != 'Z' && cmd_char != 'z') {
                // Add space before parameter unless it's negative and we're saving space
                if negative_extra_space || param >= 0.0 || j == 0 {
                    result.push(' ');
                }
            }

            result.push_str(&format_number(param, precision, leading_zero));
        }

        // Update current position
        update_position(cmd, &mut current_x, &mut current_y);
    }

    Ok(result)
}

/// Determine if absolute coordinates are more efficient
fn should_use_absolute(
    cmd: &PathCommand,
    current_x: f64,
    current_y: f64,
    precision: u8,
) -> bool {
    // Get absolute parameters
    let absolute_params = cmd.params.clone();
    
    // Get relative parameters  
    let relative_params = convert_to_relative(cmd, current_x, current_y);
    
    // Compare the string length of both representations
    let absolute_str = format_params(&absolute_params, precision, true, true);
    let relative_str = format_params(&relative_params, precision, true, true);
    
    // Use absolute if it's shorter or equal length
    // This matches SVGO's behavior where absolute is preferred when lengths are equal
    absolute_str.len() <= relative_str.len()
}

/// Format parameters as they would appear in the path string
fn format_params(params: &[f64], precision: u8, leading_zero: bool, negative_extra_space: bool) -> String {
    let mut result = String::new();
    
    for (i, &param) in params.iter().enumerate() {
        if i > 0 {
            // Add space before parameter unless it's negative and we're saving space
            if negative_extra_space || param >= 0.0 {
                result.push(' ');
            }
        }
        
        result.push_str(&format_number(param, precision, leading_zero));
    }
    
    result
}

/// Convert absolute coordinates to relative
fn convert_to_relative(cmd: &PathCommand, current_x: f64, current_y: f64) -> Vec<f64> {
    let mut params = cmd.params.clone();

    match cmd.cmd_type {
        CommandType::MoveTo | CommandType::LineTo => {
            if params.len() >= 2 {
                params[0] -= current_x;
                params[1] -= current_y;
            }
        }
        CommandType::HorizontalLineTo => {
            if !params.is_empty() {
                params[0] -= current_x;
            }
        }
        CommandType::VerticalLineTo => {
            if !params.is_empty() {
                params[0] -= current_y;
            }
        }
        CommandType::CurveTo => {
            if params.len() >= 6 {
                params[0] -= current_x;
                params[1] -= current_y;
                params[2] -= current_x;
                params[3] -= current_y;
                params[4] -= current_x;
                params[5] -= current_y;
            }
        }
        CommandType::SmoothCurveTo => {
            if params.len() >= 4 {
                params[0] -= current_x;
                params[1] -= current_y;
                params[2] -= current_x;
                params[3] -= current_y;
            }
        }
        CommandType::QuadraticBezier => {
            if params.len() >= 4 {
                params[0] -= current_x;
                params[1] -= current_y;
                params[2] -= current_x;
                params[3] -= current_y;
            }
        }
        CommandType::SmoothQuadraticBezier => {
            if params.len() >= 2 {
                params[0] -= current_x;
                params[1] -= current_y;
            }
        }
        CommandType::Arc => {
            if params.len() >= 7 {
                params[5] -= current_x;
                params[6] -= current_y;
            }
        }
        _ => {}
    }

    params
}

/// Update current position based on command
fn update_position(cmd: &PathCommand, current_x: &mut f64, current_y: &mut f64) {
    match cmd.cmd_type {
        CommandType::MoveTo | CommandType::LineTo => {
            if cmd.params.len() >= 2 {
                *current_x = cmd.params[0];
                *current_y = cmd.params[1];
            }
        }
        CommandType::HorizontalLineTo => {
            if !cmd.params.is_empty() {
                *current_x = cmd.params[0];
            }
        }
        CommandType::VerticalLineTo => {
            if !cmd.params.is_empty() {
                *current_y = cmd.params[0];
            }
        }
        CommandType::CurveTo => {
            if cmd.params.len() >= 6 {
                *current_x = cmd.params[4];
                *current_y = cmd.params[5];
            }
        }
        CommandType::SmoothCurveTo => {
            if cmd.params.len() >= 4 {
                *current_x = cmd.params[2];
                *current_y = cmd.params[3];
            }
        }
        CommandType::QuadraticBezier => {
            if cmd.params.len() >= 4 {
                *current_x = cmd.params[2];
                *current_y = cmd.params[3];
            }
        }
        CommandType::SmoothQuadraticBezier => {
            if cmd.params.len() >= 2 {
                *current_x = cmd.params[0];
                *current_y = cmd.params[1];
            }
        }
        CommandType::Arc => {
            if cmd.params.len() >= 7 {
                *current_x = cmd.params[5];
                *current_y = cmd.params[6];
            }
        }
        _ => {}
    }
}

/// Format a number with optional precision
fn format_number(value: f64, precision: u8, leading_zero: bool) -> String {
    // Format with precision
    let formatted = format!("{:.1$}", value, precision as usize);

    // Remove trailing zeros and decimal point if integer
    let mut trimmed = formatted.trim_end_matches('0').trim_end_matches('.');

    // Handle edge cases
    if trimmed.is_empty() || trimmed == "-" {
        return "0".to_string();
    }

    // Remove leading zero if requested
    if !leading_zero && trimmed.starts_with("0.") {
        trimmed = &trimmed[1..];
    } else if !leading_zero && trimmed.starts_with("-0.") {
        return format!("-{}", &trimmed[2..]);
    }

    trimmed.to_string()
}

/// Straighten curves that are nearly straight lines
fn straighten_curves(commands: Vec<PathCommand>, tolerance: f64) -> Vec<PathCommand> {
    let mut result = Vec::new();
    let mut current_x = 0.0;
    let mut current_y = 0.0;

    for cmd in commands {
        let mut new_cmd = cmd.clone();

        match cmd.cmd_type {
            CommandType::CurveTo if cmd.params.len() >= 6 => {
                // Create a cubic bezier segment
                let start = Point::new(current_x as f32, current_y as f32);
                let ctrl1 = Point::new(cmd.params[0] as f32, cmd.params[1] as f32);
                let ctrl2 = Point::new(cmd.params[2] as f32, cmd.params[3] as f32);
                let end = Point::new(cmd.params[4] as f32, cmd.params[5] as f32);
                
                let curve = CubicBezierSegment { from: start, ctrl1, ctrl2, to: end };
                
                // Check if the curve is nearly straight
                if is_curve_nearly_straight(&curve, tolerance as f32) {
                    // Convert to LineTo
                    new_cmd = PathCommand {
                        cmd_type: CommandType::LineTo,
                        is_absolute: cmd.is_absolute,
                        params: vec![cmd.params[4], cmd.params[5]],
                    };
                }
                
                current_x = cmd.params[4];
                current_y = cmd.params[5];
            }
            CommandType::QuadraticBezier if cmd.params.len() >= 4 => {
                // Create a quadratic bezier segment
                let start = Point::new(current_x as f32, current_y as f32);
                let ctrl = Point::new(cmd.params[0] as f32, cmd.params[1] as f32);
                let end = Point::new(cmd.params[2] as f32, cmd.params[3] as f32);
                
                let curve = QuadraticBezierSegment { from: start, ctrl, to: end };
                
                // Check if the curve is nearly straight
                if is_quadratic_curve_nearly_straight(&curve, tolerance as f32) {
                    // Convert to LineTo
                    new_cmd = PathCommand {
                        cmd_type: CommandType::LineTo,
                        is_absolute: cmd.is_absolute,
                        params: vec![cmd.params[2], cmd.params[3]],
                    };
                }
                
                current_x = cmd.params[2];
                current_y = cmd.params[3];
            }
            _ => {
                update_position(&cmd, &mut current_x, &mut current_y);
            }
        }

        result.push(new_cmd);
    }

    result
}

/// Convert cubic bezier curves to quadratic where possible
fn convert_cubic_to_quadratic(commands: Vec<PathCommand>, tolerance: f64) -> Vec<PathCommand> {
    let mut result = Vec::new();
    let mut current_x = 0.0;
    let mut current_y = 0.0;

    for cmd in commands {
        let mut new_cmd = cmd.clone();

        if cmd.cmd_type == CommandType::CurveTo && cmd.params.len() >= 6 {
            // Create a cubic bezier segment
            let start = Point::new(current_x as f32, current_y as f32);
            let ctrl1 = Point::new(cmd.params[0] as f32, cmd.params[1] as f32);
            let ctrl2 = Point::new(cmd.params[2] as f32, cmd.params[3] as f32);
            let end = Point::new(cmd.params[4] as f32, cmd.params[5] as f32);
            
            let curve = CubicBezierSegment { from: start, ctrl1, ctrl2, to: end };
            
            // Try to convert to quadratic
            if let Some(quad_ctrl) = cubic_to_quadratic_control_point(&curve, tolerance as f32) {
                // Convert to quadratic bezier
                new_cmd = PathCommand {
                    cmd_type: CommandType::QuadraticBezier,
                    is_absolute: cmd.is_absolute,
                    params: vec![
                        quad_ctrl.x as f64,
                        quad_ctrl.y as f64,
                        cmd.params[4],
                        cmd.params[5]
                    ],
                };
            }
            
            current_x = cmd.params[4];
            current_y = cmd.params[5];
        } else {
            update_position(&cmd, &mut current_x, &mut current_y);
        }

        result.push(new_cmd);
    }

    result
}

/// Convert curves to arcs where geometrically appropriate
fn convert_curves_to_arcs(commands: Vec<PathCommand>, tolerance: f64) -> Vec<PathCommand> {
    let mut result = Vec::new();
    let mut current_x = 0.0;
    let mut current_y = 0.0;

    for cmd in commands {
        let mut new_cmd = cmd.clone();

        match cmd.cmd_type {
            CommandType::CurveTo if cmd.params.len() >= 6 => {
                // Create a cubic bezier segment
                let start = Point::new(current_x as f32, current_y as f32);
                let ctrl1 = Point::new(cmd.params[0] as f32, cmd.params[1] as f32);
                let ctrl2 = Point::new(cmd.params[2] as f32, cmd.params[3] as f32);
                let end = Point::new(cmd.params[4] as f32, cmd.params[5] as f32);
                
                let curve = CubicBezierSegment { from: start, ctrl1, ctrl2, to: end };
                
                // Try to convert to arc
                if let Some(arc_params) = cubic_to_arc_parameters(&curve, tolerance as f32) {
                    // Convert to Arc command
                    new_cmd = PathCommand {
                        cmd_type: CommandType::Arc,
                        is_absolute: cmd.is_absolute,
                        params: arc_params,
                    };
                }
                
                current_x = cmd.params[4];
                current_y = cmd.params[5];
            }
            CommandType::QuadraticBezier if cmd.params.len() >= 4 => {
                // Create a quadratic bezier segment
                let start = Point::new(current_x as f32, current_y as f32);
                let ctrl = Point::new(cmd.params[0] as f32, cmd.params[1] as f32);
                let end = Point::new(cmd.params[2] as f32, cmd.params[3] as f32);
                
                let curve = QuadraticBezierSegment { from: start, ctrl, to: end };
                
                // Try to convert to arc
                if let Some(arc_params) = quadratic_to_arc_parameters(&curve, tolerance as f32) {
                    // Convert to Arc command
                    new_cmd = PathCommand {
                        cmd_type: CommandType::Arc,
                        is_absolute: cmd.is_absolute,
                        params: arc_params,
                    };
                }
                
                current_x = cmd.params[2];
                current_y = cmd.params[3];
            }
            _ => {
                update_position(&cmd, &mut current_x, &mut current_y);
            }
        }

        result.push(new_cmd);
    }

    result
}

/// Check if a cubic curve is nearly straight
fn is_curve_nearly_straight(curve: &CubicBezierSegment<f32>, tolerance: f32) -> bool {
    // Calculate the maximum distance from control points to the straight line
    let line_vec = curve.to - curve.from;
    let line_length = line_vec.length();
    
    if line_length < tolerance {
        return true; // Very short curve, consider it straight
    }
    
    let line_unit = line_vec / line_length;
    
    // Distance from ctrl1 to line
    let ctrl1_vec = curve.ctrl1 - curve.from;
    let ctrl1_proj = ctrl1_vec.dot(line_unit);
    let ctrl1_perp = ctrl1_vec - line_unit * ctrl1_proj;
    let ctrl1_dist = ctrl1_perp.length();
    
    // Distance from ctrl2 to line
    let ctrl2_vec = curve.ctrl2 - curve.from;
    let ctrl2_proj = ctrl2_vec.dot(line_unit);
    let ctrl2_perp = ctrl2_vec - line_unit * ctrl2_proj;
    let ctrl2_dist = ctrl2_perp.length();
    
    ctrl1_dist < tolerance && ctrl2_dist < tolerance
}

/// Check if a quadratic curve is nearly straight
fn is_quadratic_curve_nearly_straight(curve: &QuadraticBezierSegment<f32>, tolerance: f32) -> bool {
    // Calculate the distance from control point to the straight line
    let line_vec = curve.to - curve.from;
    let line_length = line_vec.length();
    
    if line_length < tolerance {
        return true; // Very short curve, consider it straight
    }
    
    let line_unit = line_vec / line_length;
    
    // Distance from ctrl to line
    let ctrl_vec = curve.ctrl - curve.from;
    let ctrl_proj = ctrl_vec.dot(line_unit);
    let ctrl_perp = ctrl_vec - line_unit * ctrl_proj;
    let ctrl_dist = ctrl_perp.length();
    
    ctrl_dist < tolerance
}

/// Try to find a quadratic control point that approximates a cubic curve
fn cubic_to_quadratic_control_point(curve: &CubicBezierSegment<f32>, tolerance: f32) -> Option<Point<f32>> {
    // For a cubic to be convertible to quadratic, the control points must be approximately collinear
    // with the start and end points, and the quadratic control point should be the intersection
    // of lines from start through ctrl1 and from end through ctrl2
    
    let start_to_ctrl1 = curve.ctrl1 - curve.from;
    let end_to_ctrl2 = curve.ctrl2 - curve.to;
    
    // Check if lines are approximately parallel (not suitable for conversion)
    let cross_product = start_to_ctrl1.x * end_to_ctrl2.y - start_to_ctrl1.y * end_to_ctrl2.x;
    if cross_product.abs() < 1e-6 {
        return None;
    }
    
    // Find intersection point using parametric line equations
    let dx = curve.to.x - curve.from.x;
    let dy = curve.to.y - curve.from.y;
    
    let det = start_to_ctrl1.x * (-end_to_ctrl2.y) - start_to_ctrl1.y * (-end_to_ctrl2.x);
    if det.abs() < 1e-6 {
        return None;
    }
    
    let t = (dx * (-end_to_ctrl2.y) - dy * (-end_to_ctrl2.x)) / det;
    
    let quad_ctrl = curve.from + start_to_ctrl1 * t;
    
    // Create a test quadratic curve and check approximation quality
    let test_quad = QuadraticBezierSegment {
        from: curve.from,
        ctrl: quad_ctrl,
        to: curve.to,
    };
    
    // Sample points and check deviation
    const SAMPLES: usize = 10;
    for i in 1..SAMPLES {
        let t = i as f32 / SAMPLES as f32;
        let cubic_point = curve.sample(t);
        let quad_point = test_quad.sample(t);
        let distance = (cubic_point - quad_point).length();
        
        if distance > tolerance {
            return None;
        }
    }
    
    Some(quad_ctrl)
}

/// Try to convert a cubic curve to arc parameters
fn cubic_to_arc_parameters(curve: &CubicBezierSegment<f32>, tolerance: f32) -> Option<Vec<f64>> {
    // This is a simplified arc detection - a full implementation would be more complex
    // For now, we'll detect if the curve could represent a circular arc
    
    // Sample points along the curve
    const SAMPLES: usize = 5;
    let mut points = Vec::new();
    for i in 0..=SAMPLES {
        let t = i as f32 / SAMPLES as f32;
        points.push(curve.sample(t));
    }
    
    // Try to fit a circle through the points
    if let Some((center, radius)) = fit_circle_to_points(&points, tolerance) {
        // Calculate arc parameters: rx, ry, x_axis_rotation, large_arc_flag, sweep_flag, x, y
        let start_angle = (curve.from.y - center.y).atan2(curve.from.x - center.x);
        let end_angle = (curve.to.y - center.y).atan2(curve.to.x - center.x);
        
        let mut angle_diff = end_angle - start_angle;
        if angle_diff > std::f32::consts::PI {
            angle_diff -= 2.0 * std::f32::consts::PI;
        } else if angle_diff < -std::f32::consts::PI {
            angle_diff += 2.0 * std::f32::consts::PI;
        }
        
        let large_arc_flag = if angle_diff.abs() > std::f32::consts::PI { 1.0 } else { 0.0 };
        let sweep_flag = if angle_diff > 0.0 { 1.0 } else { 0.0 };
        
        return Some(vec![
            radius as f64,  // rx
            radius as f64,  // ry
            0.0,            // x_axis_rotation
            large_arc_flag, // large_arc_flag
            sweep_flag,     // sweep_flag
            curve.to.x as f64, // x
            curve.to.y as f64, // y
        ]);
    }
    
    None
}

/// Try to convert a quadratic curve to arc parameters
fn quadratic_to_arc_parameters(curve: &QuadraticBezierSegment<f32>, tolerance: f32) -> Option<Vec<f64>> {
    // Sample points along the curve
    const SAMPLES: usize = 5;
    let mut points = Vec::new();
    for i in 0..=SAMPLES {
        let t = i as f32 / SAMPLES as f32;
        points.push(curve.sample(t));
    }
    
    // Try to fit a circle through the points
    if let Some((center, radius)) = fit_circle_to_points(&points, tolerance) {
        // Calculate arc parameters
        let start_angle = (curve.from.y - center.y).atan2(curve.from.x - center.x);
        let end_angle = (curve.to.y - center.y).atan2(curve.to.x - center.x);
        
        let mut angle_diff = end_angle - start_angle;
        if angle_diff > std::f32::consts::PI {
            angle_diff -= 2.0 * std::f32::consts::PI;
        } else if angle_diff < -std::f32::consts::PI {
            angle_diff += 2.0 * std::f32::consts::PI;
        }
        
        let large_arc_flag = if angle_diff.abs() > std::f32::consts::PI { 1.0 } else { 0.0 };
        let sweep_flag = if angle_diff > 0.0 { 1.0 } else { 0.0 };
        
        return Some(vec![
            radius as f64,  // rx
            radius as f64,  // ry
            0.0,            // x_axis_rotation
            large_arc_flag, // large_arc_flag
            sweep_flag,     // sweep_flag
            curve.to.x as f64, // x
            curve.to.y as f64, // y
        ]);
    }
    
    None
}

/// Fit a circle to a set of points (simplified least squares approach)
fn fit_circle_to_points(points: &[Point<f32>], tolerance: f32) -> Option<(Point<f32>, f32)> {
    if points.len() < 3 {
        return None;
    }
    
    // Use the first three points to estimate circle
    let p1 = points[0];
    let p2 = points[1];
    let p3 = points[2];
    
    // Calculate center using perpendicular bisectors
    let mid12 = Point::new((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0);
    let mid23 = Point::new((p2.x + p3.x) / 2.0, (p2.y + p3.y) / 2.0);
    
    let dir12 = Vector::new(-(p2.y - p1.y), p2.x - p1.x); // perpendicular to p1-p2
    let dir23 = Vector::new(-(p3.y - p2.y), p3.x - p2.x); // perpendicular to p2-p3
    
    // Find intersection of perpendicular bisectors
    let det = dir12.x * dir23.y - dir12.y * dir23.x;
    if det.abs() < 1e-6 {
        return None; // Lines are parallel
    }
    
    let diff = mid23 - mid12;
    let t = (diff.x * dir23.y - diff.y * dir23.x) / det;
    
    let center = mid12 + dir12 * t;
    let radius = (p1 - center).length();
    
    // Verify that all points are approximately on the circle
    for &point in points {
        let distance = (point - center).length();
        if (distance - radius).abs() > tolerance {
            return None;
        }
    }
    
    Some((center, radius))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_plugin_info() {
        let plugin = ConvertPathDataPlugin::new();
        assert_eq!(plugin.name(), "convertPathData");
        assert_eq!(
            plugin.description(),
            "converts path data to relative or absolute, optimizes segments, simplifies curves"
        );
    }

    #[test]
    fn test_param_validation() {
        let plugin = ConvertPathDataPlugin::new();

        // Test null params
        assert!(plugin.validate_params(&Value::Null).is_ok());

        // Test valid params
        assert!(plugin
            .validate_params(&json!({
                "floatPrecision": 2,
                "removeUseless": false
            }))
            .is_ok());

        // Test invalid params
        assert!(plugin
            .validate_params(&json!({
                "invalidParam": true
            }))
            .is_err());
    }

    #[test]
    fn test_parse_simple_path() {
        let path = "M10 20 L30 40";
        let commands = parse_path_data(path).unwrap();
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].cmd_type, CommandType::MoveTo);
        assert_eq!(commands[0].params, vec![10.0, 20.0]);
        assert_eq!(commands[1].cmd_type, CommandType::LineTo);
        assert_eq!(commands[1].params, vec![30.0, 40.0]);
    }

    #[test]
    fn test_parse_relative_path() {
        let path = "m10 20 l30 40";
        let commands = parse_path_data(path).unwrap();
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].cmd_type, CommandType::MoveTo);
        assert!(!commands[0].is_absolute);
        assert_eq!(commands[1].cmd_type, CommandType::LineTo);
        assert!(!commands[1].is_absolute);
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(1.0, 3, true), "1");
        assert_eq!(format_number(1.234567, 3, true), "1.235");
        assert_eq!(format_number(0.5, 1, false), ".5");
        assert_eq!(format_number(-0.5, 1, false), "-.5");
    }

    #[test]
    fn test_optimize_removes_useless_lineto() {
        let path = "M10 10 L10 10 L20 20";
        let config = ConvertPathDataConfig {
            float_precision: 3,
            transform_precision: 5,
            remove_useless: true,
            collapse_repeated: true,
            utilize_absolute: true,
            leading_zero: true,
            negative_extra_space: true,
            make_arcs: MakeArcsConfig::Enabled(false),
            straight_curves: false,
            convert_to_q: false,
            curve_tolerance: 0.1,
            arc_tolerance: 0.5,
        };
        let optimized = optimize_path_data(path, &config).unwrap();
        // Should remove the L10 10 as it's the same as current position
        assert!(!optimized.contains("L10 10"));
    }

    #[test]
    fn test_curve_straightening() {
        // Create a nearly straight cubic curve
        let curve = CubicBezierSegment {
            from: Point::new(0.0, 0.0),
            ctrl1: Point::new(1.0, 0.01), // Very slight deviation
            ctrl2: Point::new(2.0, -0.01),
            to: Point::new(3.0, 0.0),
        };
        
        assert!(is_curve_nearly_straight(&curve, 0.1));
        assert!(!is_curve_nearly_straight(&curve, 0.001));
    }

    #[test]
    fn test_quadratic_curve_straightening() {
        // Create a nearly straight quadratic curve
        let curve = QuadraticBezierSegment {
            from: Point::new(0.0, 0.0),
            ctrl: Point::new(1.5, 0.01), // Very slight deviation
            to: Point::new(3.0, 0.0),
        };
        
        assert!(is_quadratic_curve_nearly_straight(&curve, 0.1));
        assert!(!is_quadratic_curve_nearly_straight(&curve, 0.001));
    }

    #[test]
    fn test_circle_fitting() {
        // Create points on a circle
        let center = Point::new(10.0, 10.0);
        let radius = 5.0;
        let points = vec![
            Point::new(center.x + radius, center.y),
            Point::new(center.x, center.y + radius),
            Point::new(center.x - radius, center.y),
            Point::new(center.x, center.y - radius),
        ];
        
        if let Some((fitted_center, fitted_radius)) = fit_circle_to_points(&points, 0.1) {
            assert!((fitted_center.x - center.x).abs() < 0.1);
            assert!((fitted_center.y - center.y).abs() < 0.1);
            assert!((fitted_radius - radius).abs() < 0.1);
        } else {
            panic!("Should be able to fit circle to points on circle");
        }
    }

    #[test]
    fn test_advanced_config_validation() {
        let plugin = ConvertPathDataPlugin::new();

        // Test advanced features config
        assert!(plugin
            .validate_params(&json!({
                "floatPrecision": 2,
                "makeArcs": true,
                "straightCurves": true,
                "convertToQ": true,
                "curveTolerance": 0.05,
                "arcTolerance": 0.3
            }))
            .is_ok());
    }

    #[test]
    fn test_collapse_repeated_commands() {
        // Test collapsing horizontal line commands
        let commands = vec![
            PathCommand {
                cmd_type: CommandType::HorizontalLineTo,
                is_absolute: false,
                params: vec![10.0],
            },
            PathCommand {
                cmd_type: CommandType::HorizontalLineTo,
                is_absolute: false,
                params: vec![20.0],
            },
        ];
        
        let result = collapse_repeated_commands(commands);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].params[0], 30.0);
        
        // Test collapsing vertical line commands
        let commands = vec![
            PathCommand {
                cmd_type: CommandType::VerticalLineTo,
                is_absolute: false,
                params: vec![5.0],
            },
            PathCommand {
                cmd_type: CommandType::VerticalLineTo,
                is_absolute: false,
                params: vec![15.0],
            },
        ];
        
        let result = collapse_repeated_commands(commands);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].params[0], 20.0);
        
        // Test NOT collapsing commands with different signs
        let commands = vec![
            PathCommand {
                cmd_type: CommandType::HorizontalLineTo,
                is_absolute: false,
                params: vec![10.0],
            },
            PathCommand {
                cmd_type: CommandType::HorizontalLineTo,
                is_absolute: false,
                params: vec![-5.0],
            },
        ];
        
        let result = collapse_repeated_commands(commands);
        assert_eq!(result.len(), 2); // Should NOT collapse due to different signs
    }

    #[test]
    fn test_should_use_absolute() {
        // Test case where absolute is shorter
        let cmd = PathCommand {
            cmd_type: CommandType::LineTo,
            is_absolute: true,
            params: vec![5.0, 5.0],
        };
        
        // Current position is far from target, so relative will be long
        assert!(should_use_absolute(&cmd, 1000.0, 1000.0, 3));
        
        // Test case where relative is shorter
        let cmd = PathCommand {
            cmd_type: CommandType::LineTo,
            is_absolute: true,
            params: vec![1001.0, 1001.0],
        };
        
        // Current position is close to target, so relative will be short
        assert!(!should_use_absolute(&cmd, 1000.0, 1000.0, 3));
    }

    #[test]
    fn test_format_params() {
        // Test basic parameter formatting
        let params = vec![10.0, 20.0, 30.0];
        let result = format_params(&params, 3, true, true);
        assert_eq!(result, "10 20 30");
        
        // Test with negative numbers and space saving
        let params = vec![10.0, -20.0, 30.0];
        let result = format_params(&params, 3, true, false);
        assert_eq!(result, "10-20 30");
        
        // Test precision formatting
        let params = vec![10.123456, 20.999];
        let result = format_params(&params, 2, true, true);
        assert_eq!(result, "10.12 21");
    }

    #[test]
    fn test_can_collapse_commands() {
        // Test same type commands can collapse
        let cmd1 = PathCommand {
            cmd_type: CommandType::HorizontalLineTo,
            is_absolute: false,
            params: vec![10.0],
        };
        let cmd2 = PathCommand {
            cmd_type: CommandType::HorizontalLineTo,
            is_absolute: false,
            params: vec![20.0],
        };
        
        assert!(can_collapse_commands(&cmd1, &cmd2));
        
        // Test different type commands cannot collapse
        let cmd1 = PathCommand {
            cmd_type: CommandType::HorizontalLineTo,
            is_absolute: false,
            params: vec![10.0],
        };
        let cmd2 = PathCommand {
            cmd_type: CommandType::VerticalLineTo,
            is_absolute: false,
            params: vec![20.0],
        };
        
        assert!(!can_collapse_commands(&cmd1, &cmd2));
        
        // Test different absolute/relative cannot collapse
        let cmd1 = PathCommand {
            cmd_type: CommandType::HorizontalLineTo,
            is_absolute: true,
            params: vec![10.0],
        };
        let cmd2 = PathCommand {
            cmd_type: CommandType::HorizontalLineTo,
            is_absolute: false,
            params: vec![20.0],
        };
        
        assert!(!can_collapse_commands(&cmd1, &cmd2));
        
        // Test LineTo collapsing (only relative)
        let cmd1 = PathCommand {
            cmd_type: CommandType::LineTo,
            is_absolute: false,
            params: vec![10.0, 20.0],
        };
        let cmd2 = PathCommand {
            cmd_type: CommandType::LineTo,
            is_absolute: false,
            params: vec![5.0, 5.0],
        };
        
        assert!(can_collapse_commands(&cmd1, &cmd2));
        
        // Test LineTo absolute cannot collapse
        let cmd1 = PathCommand {
            cmd_type: CommandType::LineTo,
            is_absolute: true,
            params: vec![10.0, 20.0],
        };
        let cmd2 = PathCommand {
            cmd_type: CommandType::LineTo,
            is_absolute: true,
            params: vec![15.0, 25.0],
        };
        
        assert!(!can_collapse_commands(&cmd1, &cmd2));
    }
}
