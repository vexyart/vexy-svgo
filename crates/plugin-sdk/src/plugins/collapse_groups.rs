// this_file: crates/plugin-sdk/src/plugins/collapse_groups.rs

//! Collapse groups plugin implementation
//!
//! This plugin removes unnecessary group elements from SVG documents by:
//! - Collapsing empty groups without attributes
//! - Moving group attributes to single child elements when safe
//! - Removing group wrappers that serve no purpose

use crate::Plugin;
use anyhow::Result;
use std::collections::HashSet;
use vexy_svgo_core::ast::{Document, Element, Node};
use vexy_svgo_core::visitor::Visitor;
use vexy_svgo_core::error::VexyError;

/// Plugin that collapses unnecessary group elements
#[derive(Default)]
pub struct CollapseGroupsPlugin;

impl CollapseGroupsPlugin {
    /// Create a new CollapseGroupsPlugin
    pub fn new() -> Self {
        Self
    }

    /// Animation elements that prevent group collapse
    fn animation_elements() -> &'static [&'static str] {
        &[
            "animate",
            "animateColor",
            "animateMotion",
            "animateTransform",
            "set",
        ]
    }

    /// Inheritable attributes for CSS inheritance handling
    fn inheritable_attributes() -> &'static HashSet<&'static str> {
        static INHERITABLE_ATTRS: std::sync::OnceLock<HashSet<&'static str>> =
            std::sync::OnceLock::new();
        INHERITABLE_ATTRS.get_or_init(|| {
            [
                "clip-rule",
                "color",
                "color-interpolation",
                "color-interpolation-filters",
                "color-profile",
                "color-rendering",
                "cursor",
                "direction",
                "fill",
                "fill-opacity",
                "fill-rule",
                "font",
                "font-family",
                "font-size",
                "font-size-adjust",
                "font-stretch",
                "font-style",
                "font-variant",
                "font-weight",
                "glyph-orientation-horizontal",
                "glyph-orientation-vertical",
                "image-rendering",
                "kerning",
                "letter-spacing",
                "marker",
                "marker-end",
                "marker-mid",
                "marker-start",
                "pointer-events",
                "shape-rendering",
                "stroke",
                "stroke-dasharray",
                "stroke-dashoffset",
                "stroke-linecap",
                "stroke-linejoin",
                "stroke-miterlimit",
                "stroke-opacity",
                "stroke-width",
                "text-anchor",
                "text-rendering",
                "visibility",
                "word-spacing",
                "writing-mode",
            ]
            .into_iter()
            .collect()
        })
    }

    /// Check if an element contains animation children
    fn has_animation_children(element: &Element) -> bool {
        element.children.iter().any(|child| {
            if let Node::Element(elem) = child {
                Self::animation_elements().contains(&elem.name.as_ref())
            } else {
                false
            }
        })
    }

    /// Check if attributes can be safely moved from group to child
    fn can_move_attributes(group: &Element, child: &Element) -> bool {
        // Child must not have an id (to avoid reference conflicts)
        if child.attributes.contains_key("id") {
            return false;
        }

        // Group must not have filter (filters apply to group boundary)
        if group.attributes.contains_key("filter") {
            return false;
        }

        // Both cannot have class attributes (would conflict)
        if group.attributes.contains_key("class") && child.attributes.contains_key("class") {
            return false;
        }

        // Check for clip-path/mask conflicts
        if (group.attributes.contains_key("clip-path")
            && child.attributes.contains_key("clip-path"))
            || (group.attributes.contains_key("mask") && child.attributes.contains_key("mask"))
        {
            return false;
        }

        true
    }

    /// Move attributes from group to child element
    fn move_attributes<'a>(group: &Element<'a>, child: &mut Element<'a>) {
        for (attr_name, attr_value) in &group.attributes {
            match attr_name.as_ref() {
                "transform" => {
                    // Concatenate transforms: parent transform comes first
                    if let Some(child_transform) = child.attributes.get("transform") {
                        let combined = format!("{} {}", attr_value, child_transform);
                        child.attributes.insert("transform".into(), combined.into());
                    } else {
                        child
                            .attributes
                            .insert(attr_name.clone().into(), attr_value.clone().into());
                    }
                }
                _ => {
                    // Handle inheritance: replace "inherit" with parent's value
                    if let Some(existing_value) = child.attributes.get(attr_name) {
                        if existing_value == "inherit"
                            && Self::inheritable_attributes().contains(attr_name.as_ref())
                        {
                            child
                                .attributes
                                .insert(attr_name.clone(), attr_value.clone());
                        }
                        // If child already has non-inherit value, don't override
                    } else {
                        // Child doesn't have this attribute, so add it
                        child
                            .attributes
                            .insert(attr_name.clone().into(), attr_value.clone().into());
                    }
                }
            }
        }
    }

    /// Check if a group can be completely removed (empty with no attributes)
    fn can_remove_group(group: &Element, parent_name: Option<&str>) -> bool {
        // Must have no attributes
        if !group.attributes.is_empty() {
            return false;
        }

        // Must not contain animation elements
        if Self::has_animation_children(group) {
            return false;
        }

        // Cannot be direct child of switch element
        if let Some(parent) = parent_name {
            if parent == "switch" {
                return false;
            }
        }

        true
    }

    /// Process a group element for potential collapse
    fn process_group<'a>(
        group: &mut Element<'a>,
        parent_name: Option<&str>,
    ) -> Option<Vec<Node<'a>>> {
        // First check if we can remove the group entirely
        if Self::can_remove_group(group, parent_name) {
            return Some(group.children.clone());
        }

        // Check if we can move attributes to a single child
        if group.children.len() == 1 {
            if let Node::Element(child_element) = &group.children[0] {
                if Self::can_move_attributes(group, child_element) {
                    // Clone the child and move attributes to it
                    let mut new_child = child_element.clone();
                    Self::move_attributes(group, &mut new_child);
                    return Some(vec![Node::Element(new_child)]);
                }
            }
        }

        // No collapse possible
        None
    }
}

impl Plugin for CollapseGroupsPlugin {
    fn name(&self) -> &'static str {
        "collapseGroups"
    }

    fn description(&self) -> &'static str {
        "Collapse unnecessary group elements"
    }

    fn validate_params(&self, params: &serde_json::Value) -> anyhow::Result<()> {
        // This plugin doesn't accept any parameters in the original SVGO implementation
        if !params.is_null() && !params.as_object().map_or(false, |obj| obj.is_empty()) {
            return Err(anyhow::anyhow!(
                "collapseGroups plugin does not accept any parameters"
            ));
        }
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> anyhow::Result<()> {
        let mut visitor = GroupCollapseVisitor::new();
        vexy_svgo_core::visitor::walk_document(&mut visitor, document)?;
        Ok(())
    }
}

/// Visitor implementation that collapses group elements
struct GroupCollapseVisitor;

impl GroupCollapseVisitor {
    fn new() -> Self {
        Self
    }
}

impl Visitor<'_> for GroupCollapseVisitor {
    fn visit_element_exit(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        // Process from bottom up - children are already processed
        let mut indices_to_replace = Vec::new();

        for (i, child) in element.children.iter_mut().enumerate() {
            if let Node::Element(child_element) = child {
                if child_element.name.as_ref() == "g" {
                    // Try to collapse this group
                    if let Some(replacement_nodes) =
                        CollapseGroupsPlugin::process_group(child_element, Some(&element.name))
                    {
                        indices_to_replace.push((i, replacement_nodes));
                    }
                }
            }
        }

        // Apply replacements in reverse order to maintain indices
        for (index, replacement_nodes) in indices_to_replace.into_iter().rev() {
            element.children.remove(index);
            for (offset, node) in replacement_nodes.into_iter().enumerate() {
                element.children.insert(index + offset, node);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::borrow::Cow;
    use vexy_svgo_core::ast::{Document, Element, Node};

    fn create_element(name: &'static str) -> Element<'static> {
        let mut element = Element::new(name);
        element.name = Cow::Borrowed(name);
        element
    }

    fn create_element_with_attrs(name: &'static str, attrs: &[(&str, &str)]) -> Element<'static> {
        let mut element = create_element(name);
        for (key, value) in attrs {
            element.set_attr(*key, *value);
        }
        element
    }

    #[test]
    fn test_plugin_creation() {
        let plugin = CollapseGroupsPlugin::new();
        assert_eq!(plugin.name(), "collapseGroups");
    }

    #[test]
    fn test_parameter_validation() {
        let plugin = CollapseGroupsPlugin::new();

        // Valid: null parameters
        assert!(plugin.validate_params(&json!(null)).is_ok());

        // Valid: empty object
        assert!(plugin.validate_params(&json!({})).is_ok());

        // Invalid: non-empty parameters
        assert!(plugin.validate_params(&json!({"someParam": true})).is_err());
    }

    #[test]
    fn test_animation_elements() {
        assert!(CollapseGroupsPlugin::animation_elements().contains(&"animate"));
        assert!(CollapseGroupsPlugin::animation_elements().contains(&"animateTransform"));
        assert!(!CollapseGroupsPlugin::animation_elements().contains(&"rect"));
    }

    #[test]
    fn test_has_animation_children() {
        let mut group = create_element("g");
        assert!(!CollapseGroupsPlugin::has_animation_children(&group));

        // Add animation child
        group
            .children
            .push(Node::Element(create_element("animate")));
        assert!(CollapseGroupsPlugin::has_animation_children(&group));

        // Add non-animation child
        let mut group2 = create_element("g");
        group2.children.push(Node::Element(create_element("rect")));
        assert!(!CollapseGroupsPlugin::has_animation_children(&group2));
    }

    #[test]
    fn test_can_move_attributes() {
        let group = create_element("g");
        let child = create_element("rect");
        assert!(CollapseGroupsPlugin::can_move_attributes(&group, &child));

        // Child with id should prevent movement
        let child_with_id = create_element_with_attrs("rect", &[("id", "test")]);
        assert!(!CollapseGroupsPlugin::can_move_attributes(
            &group,
            &child_with_id
        ));

        // Group with filter should prevent movement
        let group_with_filter = create_element_with_attrs("g", &[("filter", "url(#filter)")]);
        assert!(!CollapseGroupsPlugin::can_move_attributes(
            &group_with_filter,
            &child
        ));

        // Both with class should prevent movement
        let group_with_class = create_element_with_attrs("g", &[("class", "group-class")]);
        let child_with_class = create_element_with_attrs("rect", &[("class", "child-class")]);
        assert!(!CollapseGroupsPlugin::can_move_attributes(
            &group_with_class,
            &child_with_class
        ));
    }

    #[test]
    fn test_move_attributes_basic() {
        let group = create_element_with_attrs("g", &[("fill", "red"), ("stroke", "blue")]);
        let mut child = create_element("rect");

        CollapseGroupsPlugin::move_attributes(&group, &mut child);

        assert_eq!(child.attr("fill"), Some("red"));
        assert_eq!(child.attr("stroke"), Some("blue"));
    }

    #[test]
    fn test_move_attributes_transform_concatenation() {
        let group = create_element_with_attrs("g", &[("transform", "translate(10,10)")]);
        let mut child = create_element_with_attrs("rect", &[("transform", "scale(2)")]);

        CollapseGroupsPlugin::move_attributes(&group, &mut child);

        assert_eq!(child.attr("transform"), Some("translate(10,10) scale(2)"));
    }

    #[test]
    fn test_move_attributes_inheritance() {
        let group = create_element_with_attrs("g", &[("fill", "red")]);
        let mut child = create_element_with_attrs("rect", &[("fill", "inherit")]);

        CollapseGroupsPlugin::move_attributes(&group, &mut child);

        // inherit should be replaced with parent's value
        assert_eq!(child.attr("fill"), Some("red"));
    }

    #[test]
    fn test_can_remove_group() {
        // Empty group with no attributes should be removable
        let group = create_element("g");
        assert!(CollapseGroupsPlugin::can_remove_group(&group, None));

        // Group with attributes should not be removable
        let group_with_attrs = create_element_with_attrs("g", &[("fill", "red")]);
        assert!(!CollapseGroupsPlugin::can_remove_group(
            &group_with_attrs,
            None
        ));

        // Group in switch should not be removable
        assert!(!CollapseGroupsPlugin::can_remove_group(
            &group,
            Some("switch")
        ));

        // Group with animation children should not be removable
        let mut group_with_animation = create_element("g");
        group_with_animation
            .children
            .push(Node::Element(create_element("animate")));
        assert!(!CollapseGroupsPlugin::can_remove_group(
            &group_with_animation,
            None
        ));
    }

    #[test]
    fn test_plugin_apply_empty_group_removal() {
        let plugin = CollapseGroupsPlugin::new();
        let mut doc = Document::new();

        // Create nested empty groups: svg > g > g > rect
        let rect = create_element("rect");
        let mut inner_group = create_element("g");
        inner_group.children.push(Node::Element(rect));
        let mut outer_group = create_element("g");
        outer_group.children.push(Node::Element(inner_group));

        doc.root.children.push(Node::Element(outer_group));

        plugin.apply(&mut doc).unwrap();

        // Both groups should be removed, leaving just the rect
        assert_eq!(doc.root.children.len(), 1);
        if let Node::Element(element) = &doc.root.children[0] {
            assert_eq!(element.name.as_ref(), "rect");
        }
    }

    #[test]
    fn test_plugin_apply_attribute_movement() {
        let plugin = CollapseGroupsPlugin::new();
        let mut doc = Document::new();

        // Create group with attributes and single child
        let child = create_element("rect");
        let mut group = create_element_with_attrs("g", &[("fill", "red"), ("stroke", "blue")]);
        group.children.push(Node::Element(child));

        doc.root.children.push(Node::Element(group));

        plugin.apply(&mut doc).unwrap();

        // Group should be collapsed, attributes moved to rect
        assert_eq!(doc.root.children.len(), 1);
        if let Node::Element(element) = &doc.root.children[0] {
            assert_eq!(element.name.as_ref(), "rect");
            assert_eq!(element.attr("fill"), Some("red"));
            assert_eq!(element.attr("stroke"), Some("blue"));
        }
    }

    #[test]
    fn test_plugin_apply_preservation() {
        let plugin = CollapseGroupsPlugin::new();
        let mut doc = Document::new();

        // Create group that should not be collapsed (multiple children)
        let mut group = create_element_with_attrs("g", &[("fill", "red")]);
        group.children.push(Node::Element(create_element("rect")));
        group.children.push(Node::Element(create_element("circle")));

        doc.root.children.push(Node::Element(group.clone()));

        plugin.apply(&mut doc).unwrap();

        // Group should be preserved
        assert_eq!(doc.root.children.len(), 1);
        if let Node::Element(element) = &doc.root.children[0] {
            assert_eq!(element.name.as_ref(), "g");
            assert_eq!(element.children.len(), 2);
            assert_eq!(element.attr("fill"), Some("red"));
        }
    }
}
