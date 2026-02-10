// this_file: crates/plugin-sdk/src/plugins/remove_hidden_elems.rs

//! Remove hidden elements plugin implementation
//!
//! This plugin removes elements that are hidden through various means:
//! - display="none"
//! - visibility="hidden" or visibility="collapse"
//! - opacity="0" (optional)
//! - Zero width/height rectangles, ellipses, images
//! - Zero radius circles
//! - Empty paths
//!
//! SVGO parameters supported:
//! - `displayNone` (default: true) - Remove elements with display="none"
//! - `opacity0` (default: true) - Remove elements with opacity="0"
//! - `circleR0` (default: true) - Remove circles with r="0"
//! - `ellipseRX0` (default: true) - Remove ellipses with rx="0"
//! - `ellipseRY0` (default: true) - Remove ellipses with ry="0"
//! - `rectWidth0` (default: true) - Remove rects with width="0"
//! - `rectHeight0` (default: true) - Remove rects with height="0"
//! - `patternWidth0` (default: true) - Remove patterns with width="0"
//! - `patternHeight0` (default: true) - Remove patterns with height="0"
//! - `imageWidth0` (default: true) - Remove images with width="0"
//! - `imageHeight0` (default: true) - Remove images with height="0"
//! - `pathEmptyD` (default: true) - Remove paths with empty d attribute
//! - `polylineEmptyPoints` (default: true) - Remove polylines with empty points
//! - `polygonEmptyPoints` (default: true) - Remove polygons with empty points

use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vexy_svgo_core::ast::{Document, Element, Node};
use vexy_svgo_core::error::VexyError;
use vexy_svgo_core::visitor::Visitor;

use crate::Plugin;

/// Configuration parameters for remove hidden elems plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveHiddenElemsConfig {
    /// Remove elements with display="none"
    #[serde(default = "default_true")]
    pub display_none: bool,

    /// Remove elements with opacity="0"
    #[serde(default = "default_true")]
    pub opacity0: bool,

    /// Remove circles with r="0"
    #[serde(default = "default_true")]
    pub circle_r0: bool,

    /// Remove ellipses with rx="0"
    #[serde(default = "default_true")]
    pub ellipse_rx0: bool,

    /// Remove ellipses with ry="0"
    #[serde(default = "default_true")]
    pub ellipse_ry0: bool,

    /// Remove rects with width="0"
    #[serde(default = "default_true")]
    pub rect_width0: bool,

    /// Remove rects with height="0"
    #[serde(default = "default_true")]
    pub rect_height0: bool,

    /// Remove patterns with width="0"
    #[serde(default = "default_true")]
    pub pattern_width0: bool,

    /// Remove patterns with height="0"
    #[serde(default = "default_true")]
    pub pattern_height0: bool,

    /// Remove images with width="0"
    #[serde(default = "default_true")]
    pub image_width0: bool,

    /// Remove images with height="0"
    #[serde(default = "default_true")]
    pub image_height0: bool,

    /// Remove paths with empty d attribute
    #[serde(default = "default_true")]
    pub path_empty_d: bool,

    /// Remove polylines with empty points
    #[serde(default = "default_true")]
    pub polyline_empty_points: bool,

    /// Remove polygons with empty points
    #[serde(default = "default_true")]
    pub polygon_empty_points: bool,
}

impl Default for RemoveHiddenElemsConfig {
    fn default() -> Self {
        Self {
            display_none:          default_true(),
            opacity0:              default_true(),
            circle_r0:             default_true(),
            ellipse_rx0:           default_true(),
            ellipse_ry0:           default_true(),
            rect_width0:           default_true(),
            rect_height0:          default_true(),
            pattern_width0:        default_true(),
            pattern_height0:       default_true(),
            image_width0:          default_true(),
            image_height0:         default_true(),
            path_empty_d:          default_true(),
            polyline_empty_points: default_true(),
            polygon_empty_points:  default_true(),
        }
    }
}

fn default_true() -> bool {
    true
}

/// Plugin that removes hidden elements
pub struct RemoveHiddenElemsPlugin {
    config: RemoveHiddenElemsConfig,
}

impl RemoveHiddenElemsPlugin {
    /// Create a new RemoveHiddenElemsPlugin
    pub fn new() -> Self {
        Self {
            config: RemoveHiddenElemsConfig::default(),
        }
    }

    /// Create a new RemoveHiddenElemsPlugin with config
    pub fn with_config(config: RemoveHiddenElemsConfig) -> Self {
        Self { config }
    }

    /// Parse configuration from JSON
    fn _parse_config(params: &Value) -> Result<RemoveHiddenElemsConfig> {
        if let Some(_obj) = params.as_object() {
            serde_json::from_value(params.clone()).map_err(|e| anyhow!("Invalid configuration: {}", e))
        } else {
            Ok(RemoveHiddenElemsConfig::default())
        }
    }
}

impl Default for RemoveHiddenElemsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RemoveHiddenElemsPlugin {
    fn name(&self) -> &'static str {
        "removeHiddenElems"
    }

    fn description(&self) -> &'static str {
        "Remove hidden elements"
    }

    fn validate_params(&self, params: &Value) -> Result<()> {
        if let Some(obj) = params.as_object() {
            // Validate all boolean parameters
            for (key, value) in obj {
                match key.as_str() {
                    "displayNone"
                    | "opacity0"
                    | "circleR0"
                    | "ellipseRX0"
                    | "ellipseRY0"
                    | "rectWidth0"
                    | "rectHeight0"
                    | "patternWidth0"
                    | "patternHeight0"
                    | "imageWidth0"
                    | "imageHeight0"
                    | "pathEmptyD"
                    | "polylineEmptyPoints"
                    | "polygonEmptyPoints" => {
                        if !value.is_boolean() {
                            return Err(anyhow!("{} must be a boolean", key));
                        }
                    },
                    _ => return Err(anyhow!("Unknown parameter: {}", key)),
                }
            }
        }
        Ok(())
    }

    fn configure(&mut self, params: &Value) -> Result<()> {
        self.config = Self::_parse_config(params)?;
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> Result<()> {
        // Phase 1: collect <use> references by target id.
        let mut use_references_by_id: HashMap<String, usize> = HashMap::new();
        collect_use_references(&document.root, &mut use_references_by_id);

        // Phase 2: collect existing <defs> count.
        let mut defs_count = 0usize;
        collect_defs_count(&document.root, &mut defs_count);

        let mut referenced_ids = HashSet::new();
        collect_referenced_ids(&document.root, &mut referenced_ids);

        let mut style_collector = HiddenElemsStyleCollector::new();
        vexy_svgo_core::visitor::walk_document(&mut style_collector, document)?;

        let mut visitor =
            HiddenElemsRemovalVisitor::new(self.config.clone(), style_collector.class_overrides, referenced_ids);
        // Phase 3: run hidden-element removal visitor.
        vexy_svgo_core::visitor::walk_document(&mut visitor, document)?;

        // Phase 4: collect ids removed from inside <defs>.
        let removed_def_ids = visitor.take_removed_def_ids();

        // Phase 5: remove <use> elements that reference removed definitions.
        if !removed_def_ids.is_empty() {
            let mut removed_use_target_ids = HashSet::new();
            for id in &removed_def_ids {
                if use_references_by_id.contains_key(id) {
                    removed_use_target_ids.insert(id.clone());
                }
            }

            if !removed_use_target_ids.is_empty() {
                remove_use_elements_referencing_ids(&mut document.root, &removed_use_target_ids);
            }
        }

        // Phase 6: remove empty <defs> containers.
        if defs_count > 0 {
            remove_empty_defs(&mut document.root);
        }

        normalize_textpath_indentation(&mut document.root);
        cleanup_document_whitespace(document);

        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
struct ClassStyleOverride {
    display:    Option<String>,
    opacity:    Option<f64>,
    visibility: Option<String>,
}

struct HiddenElemsStyleCollector {
    class_overrides: HashMap<String, ClassStyleOverride>,
}

impl HiddenElemsStyleCollector {
    fn new() -> Self {
        Self {
            class_overrides: HashMap::new(),
        }
    }

    fn parse_style_text(&mut self, css_text: &str) {
        for block in css_text.split('}') {
            if let Some((selectors, body)) = block.split_once('{') {
                for selector in selectors.split(',') {
                    let selector = selector.trim();
                    if !selector.starts_with('.') {
                        continue;
                    }

                    let class_name = selector
                        .trim_start_matches('.')
                        .split(|c: char| c == ':' || c == ' ' || c == '[' || c == '>')
                        .next()
                        .unwrap_or("")
                        .trim();

                    if class_name.is_empty() {
                        continue;
                    }

                    let entry = self.class_overrides.entry(class_name.to_string()).or_default();

                    for declaration in body.split(';') {
                        if let Some((name, value)) = declaration.split_once(':') {
                            let name = name.trim();
                            let value = value.trim();
                            match name {
                                "display" => entry.display = Some(value.to_string()),
                                "visibility" => entry.visibility = Some(value.to_string()),
                                "opacity" => {
                                    if let Ok(v) = value.parse::<f64>() {
                                        entry.opacity = Some(v);
                                    }
                                },
                                _ => {},
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Visitor<'_> for HiddenElemsStyleCollector {
    fn visit_element_enter(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        if element.name == "style" {
            for child in &element.children {
                match child {
                    Node::Text(text) | Node::CData(text) => self.parse_style_text(text),
                    _ => {},
                }
            }
        }
        Ok(())
    }
}

/// Visitor implementation that removes hidden elements
struct HiddenElemsRemovalVisitor {
    config:          RemoveHiddenElemsConfig,
    class_overrides: HashMap<String, ClassStyleOverride>,
    referenced_ids:  HashSet<String>,
    removed_def_ids: HashSet<String>,
}

impl HiddenElemsRemovalVisitor {
    fn new(
        config: RemoveHiddenElemsConfig,
        class_overrides: HashMap<String, ClassStyleOverride>,
        referenced_ids: HashSet<String>,
    ) -> Self {
        Self {
            config,
            class_overrides,
            referenced_ids,
            removed_def_ids: HashSet::new(),
        }
    }

    fn take_removed_def_ids(&mut self) -> HashSet<String> {
        std::mem::take(&mut self.removed_def_ids)
    }

    fn is_empty_container(&self, element: &Element) -> bool {
        !element.children.iter().any(|child| matches!(child, Node::Element(_)))
    }

    fn is_empty_gradient(&self, element: &Element) -> bool {
        matches!(element.name.as_ref(), "linearGradient" | "radialGradient") && self.is_empty_container(element)
    }

    fn has_referenced_id(&self, element: &Element) -> bool {
        element.attributes.get("id").map(|id| self.referenced_ids.contains(id.as_ref())).unwrap_or(false)
    }

    fn has_class_override(&self, element: &Element, property: &str) -> bool {
        let Some(class_value) = element.attributes.get("class") else {
            return false;
        };

        for class_name in class_value.split_whitespace() {
            if let Some(overrides) = self.class_overrides.get(class_name) {
                match property {
                    "display" => {
                        if let Some(display) = &overrides.display {
                            if display != "none" {
                                return true;
                            }
                        }
                    },
                    "visibility" => {
                        if let Some(visibility) = &overrides.visibility {
                            if visibility != "hidden" && visibility != "collapse" {
                                return true;
                            }
                        }
                    },
                    "opacity" => {
                        if let Some(opacity) = overrides.opacity {
                            if opacity > 0.0 {
                                return true;
                            }
                        }
                    },
                    _ => {},
                }
            }
        }

        false
    }

    fn has_visible_descendant(&self, element: &Element) -> bool {
        for child in &element.children {
            if let Node::Element(child_element) = child {
                if child_element.attributes.get("visibility").map(|value| value == "visible").unwrap_or(false)
                    || self.has_class_override(child_element, "visibility")
                {
                    return true;
                }

                if self.has_visible_descendant(child_element) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if an element is hidden and should be removed
    fn is_element_hidden(&self, element: &Element, parent_name: Option<&str>) -> bool {
        // Check display="none"
        if self.config.display_none {
            if let Some(display) = element.attributes.get("display") {
                if display == "none" && !self.has_class_override(element, "display") {
                    return true;
                }
            }
        }

        // Check visibility="hidden" or visibility="collapse"
        if let Some(visibility) = element.attributes.get("visibility") {
            if (visibility == "hidden" || visibility == "collapse")
                && !self.has_class_override(element, "visibility")
                && !self.has_visible_descendant(element)
            {
                return true;
            }
        }

        // Check opacity="0"
        if self.config.opacity0 {
            let inside_non_rendering_parent = parent_name.map(is_non_rendering_element_name).unwrap_or(false);
            if !inside_non_rendering_parent {
                let has_opacity_zero_attr = element
                    .attributes
                    .get("opacity")
                    .and_then(|opacity| opacity.parse::<f64>().ok())
                    .map(|opacity_val| opacity_val == 0.0)
                    .unwrap_or(false);

                let has_opacity_zero = has_opacity_zero_attr || self.has_inline_style_opacity_zero(element);
                if has_opacity_zero && !self.has_class_override(element, "opacity") && !self.has_referenced_id(element)
                {
                    return true;
                }
            }
        }

        // Check element-specific zero dimensions
        match element.name.as_ref() {
            "linearGradient" | "radialGradient" => {
                if self.is_empty_gradient(element) && !self.has_referenced_id(element) {
                    return true;
                }
            },
            "circle" if self.config.circle_r0 => {
                if element.children.is_empty() && self.is_zero_dimension(element, "r") {
                    return true;
                }
            },
            "ellipse" => {
                if element.children.is_empty()
                    && ((self.config.ellipse_rx0 && self.is_zero_dimension(element, "rx"))
                        || (self.config.ellipse_ry0 && self.is_zero_dimension(element, "ry")))
                {
                    return true;
                }
            },
            "rect" => {
                if element.children.is_empty()
                    && ((self.config.rect_width0 && self.is_zero_dimension(element, "width"))
                        || (self.config.rect_height0 && self.is_zero_dimension(element, "height")))
                {
                    return true;
                }
            },
            "pattern" => {
                if (self.config.pattern_width0 && self.is_zero_dimension(element, "width"))
                    || (self.config.pattern_height0 && self.is_zero_dimension(element, "height"))
                {
                    return true;
                }
            },
            "image" => {
                if (self.config.image_width0 && self.is_zero_dimension(element, "width"))
                    || (self.config.image_height0 && self.is_zero_dimension(element, "height"))
                {
                    return true;
                }
            },
            "path" if self.config.path_empty_d => {
                if let Some(d) = element.attributes.get("d") {
                    if d.trim().is_empty() {
                        return true;
                    }

                    if is_invalid_path_data(d) {
                        return true;
                    }

                    if is_single_point_path(d)
                        && !element.attributes.contains_key("marker-start")
                        && !element.attributes.contains_key("marker-end")
                    {
                        return true;
                    }
                } else {
                    return true;
                }
            },
            "polyline" if self.config.polyline_empty_points => {
                if let Some(points) = element.attributes.get("points") {
                    if points.trim().is_empty() {
                        return true;
                    }
                } else {
                    return true;
                }
            },
            "polygon" if self.config.polygon_empty_points => {
                if let Some(points) = element.attributes.get("points") {
                    if points.trim().is_empty() {
                        return true;
                    }
                } else {
                    return true;
                }
            },
            "line" => {
                // Line is hidden if start and end points are the same
                let x1 = self.get_numeric_attr(element, "x1").unwrap_or(0.0);
                let y1 = self.get_numeric_attr(element, "y1").unwrap_or(0.0);
                let x2 = self.get_numeric_attr(element, "x2").unwrap_or(0.0);
                let y2 = self.get_numeric_attr(element, "y2").unwrap_or(0.0);
                if x1 == x2 && y1 == y2 {
                    return true;
                }
            },
            _ => {},
        }

        false
    }

    fn is_zero_dimension(&self, element: &Element, attr_name: &str) -> bool {
        if let Some(value) = element.attributes.get(attr_name) {
            if let Ok(num_val) = value.parse::<f64>() {
                return num_val == 0.0;
            }
        }
        false
    }

    fn get_numeric_attr(&self, element: &Element, attr_name: &str) -> Option<f64> {
        element.attributes.get(attr_name)?.parse::<f64>().ok()
    }

    fn has_inline_style_opacity_zero(&self, element: &Element) -> bool {
        let Some(style_value) = element.attributes.get("style") else {
            return false;
        };

        for declaration in style_value.split(';') {
            if let Some((name, value)) = declaration.split_once(':') {
                if name.trim() == "opacity" {
                    if let Ok(opacity_val) = value.trim().parse::<f64>() {
                        if opacity_val == 0.0 {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }
}

fn collect_referenced_ids(element: &Element<'_>, referenced_ids: &mut HashSet<String>) {
    for value in element.attributes.values() {
        collect_ids_from_attr_value(value, referenced_ids);
    }

    for child in &element.children {
        if let Node::Element(child_element) = child {
            collect_referenced_ids(child_element, referenced_ids);
        }
    }
}

fn collect_ids_from_attr_value(value: &str, referenced_ids: &mut HashSet<String>) {
    let bytes = value.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        if bytes[idx] == b'#' {
            let start = idx + 1;
            let mut end = start;
            while end < bytes.len() {
                let ch = bytes[end] as char;
                if ch.is_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.') {
                    end += 1;
                } else {
                    break;
                }
            }

            if end > start {
                referenced_ids.insert(value[start..end].to_string());
                idx = end;
                continue;
            }
        }
        idx += 1;
    }
}

fn collect_use_references(element: &Element<'_>, references_by_id: &mut HashMap<String, usize>) {
    if element.name == "use" {
        for attr_name in ["href", "xlink:href"] {
            if let Some(value) = element.attributes.get(attr_name) {
                if let Some(id) = value.strip_prefix('#') {
                    if !id.is_empty() {
                        let counter = references_by_id.entry(id.to_string()).or_insert(0);
                        *counter += 1;
                    }
                }
            }
        }
    }

    for child in &element.children {
        if let Node::Element(child_element) = child {
            collect_use_references(child_element, references_by_id);
        }
    }
}

fn collect_defs_count(element: &Element<'_>, defs_count: &mut usize) {
    if element.name == "defs" {
        *defs_count += 1;
    }

    for child in &element.children {
        if let Node::Element(child_element) = child {
            collect_defs_count(child_element, defs_count);
        }
    }
}

fn remove_use_elements_referencing_ids(element: &mut Element<'_>, removed_ids: &HashSet<String>) {
    let mut idx = 0;
    while idx < element.children.len() {
        let should_remove = match &element.children[idx] {
            Node::Element(child_element) if child_element.name == "use" => {
                references_removed_id(child_element, removed_ids)
            },
            _ => false,
        };

        if should_remove {
            element.children.remove(idx);
            continue;
        }

        if let Node::Element(child_element) = &mut element.children[idx] {
            remove_use_elements_referencing_ids(child_element, removed_ids);
        }
        idx += 1;
    }
}

fn references_removed_id(element: &Element<'_>, removed_ids: &HashSet<String>) -> bool {
    for attr_name in ["href", "xlink:href"] {
        if let Some(value) = element.attributes.get(attr_name) {
            if let Some(id) = value.strip_prefix('#') {
                if removed_ids.contains(id) {
                    return true;
                }
            }
        }
    }

    false
}

fn remove_empty_defs(element: &mut Element<'_>) {
    let mut idx = 0;
    while idx < element.children.len() {
        if let Node::Element(child_element) = &mut element.children[idx] {
            remove_empty_defs(child_element);
        }

        let should_remove = match &element.children[idx] {
            Node::Element(child_element) => {
                child_element.name == "defs"
                    && !child_element.children.iter().any(|child| matches!(child, Node::Element(_)))
            },
            _ => false,
        };

        if should_remove {
            element.children.remove(idx);
            continue;
        }

        idx += 1;
    }
}

fn normalize_textpath_indentation(element: &mut Element<'_>) {
    if element.name == "textPath" {
        for child in &mut element.children {
            if let Node::Text(text) = child {
                *text = dedent_one_level(text).into();
            }
        }
    }

    for child in &mut element.children {
        if let Node::Element(child_element) = child {
            normalize_textpath_indentation(child_element);
        }
    }
}

fn dedent_one_level(text: &str) -> String {
    let mut lines = Vec::new();
    for line in text.split('\n') {
        if let Some(stripped) = line.strip_prefix("    ") {
            lines.push(stripped);
        } else {
            lines.push(line);
        }
    }
    lines.join("\n")
}

fn cleanup_document_whitespace(document: &mut Document<'_>) {
    document.prologue.retain(|node| !matches!(node, Node::Text(text) if text.trim().is_empty()));
    document.epilogue.clear();
}

fn is_single_point_path(path_data: &str) -> bool {
    let mut command_positions = Vec::new();

    for (idx, ch) in path_data.char_indices() {
        if matches!(
            ch,
            'M' | 'm'
                | 'L'
                | 'l'
                | 'H'
                | 'h'
                | 'V'
                | 'v'
                | 'C'
                | 'c'
                | 'S'
                | 's'
                | 'Q'
                | 'q'
                | 'T'
                | 't'
                | 'A'
                | 'a'
                | 'Z'
                | 'z'
        ) {
            command_positions.push((idx, ch));
        }
    }

    if command_positions.len() != 1 {
        return false;
    }

    let (_, command) = command_positions[0];
    if !matches!(command, 'M' | 'm') {
        return false;
    }

    let params_start = command_positions[0].0 + command.len_utf8();
    let params = &path_data[params_start..];
    let numbers_count = count_numeric_tokens(params);

    // moveto consumes one coordinate pair; additional pairs imply implicit lineto.
    if numbers_count > 2 {
        return false;
    }

    numbers_count == 2
}

fn is_invalid_path_data(path_data: &str) -> bool {
    let mut command_positions = Vec::new();

    for (idx, ch) in path_data.char_indices() {
        if matches!(
            ch,
            'M' | 'm'
                | 'L'
                | 'l'
                | 'H'
                | 'h'
                | 'V'
                | 'v'
                | 'C'
                | 'c'
                | 'S'
                | 's'
                | 'Q'
                | 'q'
                | 'T'
                | 't'
                | 'A'
                | 'a'
                | 'Z'
                | 'z'
        ) {
            command_positions.push((idx, ch));
        }
    }

    if command_positions.is_empty() {
        return true;
    }

    let mut has_move_to = false;

    for idx in 0..command_positions.len() {
        let (start, command) = command_positions[idx];
        let end = if idx + 1 < command_positions.len() {
            command_positions[idx + 1].0
        } else {
            path_data.len()
        };

        let params = &path_data[start + command.len_utf8()..end];
        let numbers_count = count_numeric_tokens(params);

        let valid = match command {
            'M' | 'm' => {
                has_move_to = true;
                numbers_count >= 2 && numbers_count % 2 == 0
            },
            'Z' | 'z' => has_move_to,
            'L' | 'l' | 'T' | 't' => numbers_count >= 2 && numbers_count % 2 == 0,
            'H' | 'h' | 'V' | 'v' => numbers_count >= 1,
            'S' | 's' | 'Q' | 'q' => numbers_count >= 4 && numbers_count % 4 == 0,
            'C' | 'c' => numbers_count >= 6 && numbers_count % 6 == 0,
            'A' | 'a' => numbers_count >= 7 && numbers_count % 7 == 0,
            _ => false,
        };

        if !valid {
            return true;
        }
    }

    false
}

fn count_numeric_tokens(input: &str) -> usize {
    let bytes = input.as_bytes();
    let mut idx = 0;
    let mut count = 0;

    while idx < bytes.len() {
        let ch = bytes[idx] as char;
        if ch.is_ascii_whitespace() || ch == ',' {
            idx += 1;
            continue;
        }

        let is_number_start = ch.is_ascii_digit() || ch == '.' || ch == '-' || ch == '+';
        if !is_number_start {
            idx += 1;
            continue;
        }

        count += 1;
        idx += 1;

        let mut seen_dot = ch == '.';
        while idx < bytes.len() {
            let token_ch = bytes[idx] as char;
            if token_ch.is_ascii_digit() {
                idx += 1;
                continue;
            }

            if token_ch == '.' {
                if seen_dot {
                    break;
                }
                seen_dot = true;
                idx += 1;
                continue;
            }

            if token_ch == 'e' || token_ch == 'E' {
                idx += 1;
                if idx < bytes.len() {
                    let sign = bytes[idx] as char;
                    if sign == '-' || sign == '+' {
                        idx += 1;
                    }
                }
                continue;
            }

            break;
        }
    }

    count
}

fn is_non_rendering_element_name(name: &str) -> bool {
    matches!(
        name,
        "defs" | "clipPath" | "mask" | "marker" | "pattern" | "linearGradient" | "radialGradient" | "symbol"
    )
}

impl Visitor<'_> for HiddenElemsRemovalVisitor {
    fn visit_element_exit(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        // Remove hidden child elements (process children first)
        element.children.retain(|child| {
            if let Node::Element(child_element) = child {
                let should_remove = self.is_element_hidden(child_element, Some(element.name.as_ref()));
                if should_remove && element.name == "defs" {
                    if let Some(id) = child_element.attributes.get("id") {
                        self.removed_def_ids.insert(id.to_string());
                    }
                }
                !should_remove
            } else {
                true // Keep non-element nodes
            }
        });

        Ok(())
    }
}

#[cfg(test)]
mod unit_tests {
    use std::borrow::Cow;

    use serde_json::json;
    use vexy_svgo_core::ast::{Document, Element, Node};

    use super::*;

    fn create_element(name: &'static str) -> Element<'static> {
        let mut element = Element::new(name);
        element.name = Cow::Borrowed(name);
        element
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = RemoveHiddenElemsPlugin::new();
        assert_eq!(plugin.name(), "removeHiddenElems");
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = RemoveHiddenElemsPlugin::new();

        // Valid parameters
        assert!(plugin.validate_params(&json!({})).is_ok());
        assert!(plugin
            .validate_params(&json!({
                "displayNone": true,
                "opacity0": false,
                "circleR0": true
            }))
            .is_ok());

        // Invalid parameters
        assert!(plugin.validate_params(&json!({"displayNone": "invalid"})).is_err());
        assert!(plugin.validate_params(&json!({"unknownParam": true})).is_err());
    }

    #[test]
    fn test_remove_display_none() {
        let plugin = RemoveHiddenElemsPlugin::new();
        let mut doc = Document::new();

        // Add elements with display attributes
        let mut rect1 = create_element("rect");
        rect1.attributes.insert("display".into(), "none".into());
        rect1.attributes.insert("width".into(), "100".into());

        let mut rect2 = create_element("rect");
        rect2.attributes.insert("display".into(), "block".into());
        rect2.attributes.insert("width".into(), "100".into());

        doc.root.children.push(Node::Element(rect1));
        doc.root.children.push(Node::Element(rect2));

        plugin.apply(&mut doc).unwrap();

        // Only the visible rect should remain
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(elem)) = doc.root.children.get(0) {
            assert_eq!(elem.attr("display"), Some("block"));
        }
    }

    #[test]
    fn test_remove_visibility_hidden() {
        let plugin = RemoveHiddenElemsPlugin::new();
        let mut doc = Document::new();

        // Add elements with visibility attributes
        let mut rect1 = create_element("rect");
        rect1.set_attr("visibility", "hidden");

        let mut rect2 = create_element("rect");
        rect2.set_attr("visibility", "collapse");

        let mut rect3 = create_element("rect");
        rect3.set_attr("visibility", "visible");

        doc.root.children.push(Node::Element(rect1));
        doc.root.children.push(Node::Element(rect2));
        doc.root.children.push(Node::Element(rect3));

        plugin.apply(&mut doc).unwrap();

        // Only the visible rect should remain
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(elem)) = doc.root.children.get(0) {
            assert_eq!(elem.attr("visibility"), Some("visible"));
        }
    }

    #[test]
    fn test_remove_opacity_zero() {
        let plugin = RemoveHiddenElemsPlugin::new();
        let mut doc = Document::new();

        // Add elements with opacity
        let mut rect1 = create_element("rect");
        rect1.set_attr("opacity", "0");

        let mut rect2 = create_element("rect");
        rect2.set_attr("opacity", "0.0");

        let mut rect3 = create_element("rect");
        rect3.set_attr("opacity", "0.5");

        doc.root.children.push(Node::Element(rect1));
        doc.root.children.push(Node::Element(rect2));
        doc.root.children.push(Node::Element(rect3));

        plugin.apply(&mut doc).unwrap();

        // Only the non-zero opacity rect should remain
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(elem)) = doc.root.children.get(0) {
            assert_eq!(elem.attr("opacity"), Some("0.5"));
        }
    }

    #[test]
    fn test_remove_zero_dimension_circle() {
        let plugin = RemoveHiddenElemsPlugin::new();
        let mut doc = Document::new();

        // Add circles with different radii
        let mut circle1 = create_element("circle");
        circle1.set_attr("r", "0");

        let mut circle2 = create_element("circle");
        circle2.set_attr("r", "10");

        doc.root.children.push(Node::Element(circle1));
        doc.root.children.push(Node::Element(circle2));

        plugin.apply(&mut doc).unwrap();

        // Only non-zero radius circle should remain
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(elem)) = doc.root.children.get(0) {
            assert_eq!(elem.attr("r"), Some("10"));
        }
    }

    #[test]
    fn test_remove_zero_dimension_rect() {
        let plugin = RemoveHiddenElemsPlugin::new();
        let mut doc = Document::new();

        // Add rects with zero dimensions
        let mut rect1 = create_element("rect");
        rect1.set_attr("width", "0");
        rect1.set_attr("height", "100");

        let mut rect2 = create_element("rect");
        rect2.set_attr("width", "100");
        rect2.set_attr("height", "0");

        let mut rect3 = create_element("rect");
        rect3.set_attr("width", "100");
        rect3.set_attr("height", "100");

        doc.root.children.push(Node::Element(rect1));
        doc.root.children.push(Node::Element(rect2));
        doc.root.children.push(Node::Element(rect3));

        plugin.apply(&mut doc).unwrap();

        // Only non-zero dimension rect should remain
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(elem)) = doc.root.children.get(0) {
            assert_eq!(elem.attr("width"), Some("100"));
            assert_eq!(elem.attr("height"), Some("100"));
        }
    }

    #[test]
    fn test_remove_empty_path() {
        let plugin = RemoveHiddenElemsPlugin::new();
        let mut doc = Document::new();

        // Add paths with different d attributes
        let mut path1 = create_element("path");
        path1.set_attr("d", "");

        let mut path2 = create_element("path");
        path2.set_attr("d", "   ");

        let mut path3 = create_element("path");
        path3.set_attr("d", "M10 10 L20 20");

        let path4 = create_element("path"); // No d attribute

        doc.root.children.push(Node::Element(path1));
        doc.root.children.push(Node::Element(path2));
        doc.root.children.push(Node::Element(path3));
        doc.root.children.push(Node::Element(path4));

        plugin.apply(&mut doc).unwrap();

        // Only path with valid d should remain
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(elem)) = doc.root.children.get(0) {
            assert_eq!(elem.attr("d"), Some("M10 10 L20 20"));
        }
    }

    #[test]
    fn test_config_parsing() {
        let config = RemoveHiddenElemsPlugin::_parse_config(&json!({
            "displayNone": false,
            "opacity0": true,
            "circleR0": false,
            "pathEmptyD": true
        }))
        .unwrap();

        assert_eq!(config.display_none, false);
        assert_eq!(config.opacity0, true);
        assert_eq!(config.circle_r0, false);
        assert_eq!(config.path_empty_d, true);
    }

    #[test]
    fn test_line_same_coordinates() {
        let plugin = RemoveHiddenElemsPlugin::new();
        let mut doc = Document::new();

        // Add lines with same start/end points
        let mut line1 = create_element("line");
        line1.attributes.insert("x1".into(), "10".into());
        line1.attributes.insert("y1".into(), "10".into());
        line1.attributes.insert("x2".into(), "10".into());
        line1.attributes.insert("y2".into(), "10".into());

        let mut line2 = create_element("line");
        line2.attributes.insert("x1".into(), "10".into());
        line2.attributes.insert("y1".into(), "10".into());
        line2.attributes.insert("x2".into(), "20".into());
        line2.attributes.insert("y2".into(), "20".into());

        doc.root.children.push(Node::Element(line1));
        doc.root.children.push(Node::Element(line2));

        plugin.apply(&mut doc).unwrap();

        // Only line with different coordinates should remain
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(elem)) = doc.root.children.get(0) {
            assert_eq!(elem.attr("x2"), Some("20"));
        }
    }

    #[test]
    fn test_selective_removal_config() {
        let config = RemoveHiddenElemsConfig {
            display_none: false,
            opacity0: true,
            ..Default::default()
        };
        let plugin = RemoveHiddenElemsPlugin::with_config(config);
        let mut doc = Document::new();

        // Add elements that would normally be removed
        let mut rect1 = create_element("rect");
        rect1.attributes.insert("display".into(), "none".into());

        let mut rect2 = create_element("rect");
        rect2.attributes.insert("opacity".into(), "0".into());

        doc.root.children.push(Node::Element(rect1));
        doc.root.children.push(Node::Element(rect2));

        plugin.apply(&mut doc).unwrap();

        // display="none" should be kept, opacity="0" should be removed
        assert_eq!(doc.root.children.len(), 1);
        if let Some(Node::Element(elem)) = doc.root.children.get(0) {
            assert_eq!(elem.attr("display"), Some("none"));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use vexy_svgo_core::Config;
    use vexy_svgo_test_utils::load_fixtures;

    #[test]
    fn fixture_tests() -> Result<(), Box<dyn std::error::Error>> {
        let fixtures_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("testdata")
            .join("plugins")
            .join("removeHiddenElems");

        let fixtures = load_fixtures(&fixtures_path)?;
        for fixture in fixtures {
            let mut config = Config::new();
            config.plugins = vec![vexy_svgo_core::PluginConfig::Name("removeHiddenElems".to_string())];
            config.js2svg.pretty = true;
            config.js2svg.indent = "    ".to_string();
            config.js2svg.final_newline = false;

            let registry = crate::registry::create_migrated_plugin_registry();
            let options = vexy_svgo_core::OptimizeOptions::new(config).with_registry(registry);
            let result = vexy_svgo_core::optimize(&fixture.input, options)?;

            assert_eq!(result.data, fixture.expected, "Fixture: {}", fixture.name);
        }

        Ok(())
    }
}
