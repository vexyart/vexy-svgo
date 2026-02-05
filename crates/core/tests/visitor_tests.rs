// this_file: crates/core/tests/visitor_tests.rs

//! Comprehensive tests for visitor pattern functionality

use vexy_svgo_core::{
    ast::{Document, Element, Node},
    visitor::{Visitor, walk_document, walk_element, walk_node},
    parse_svg, VexyError,
};
use std::collections::HashMap;

// Test visitor that counts elements
struct ElementCounter {
    counts: HashMap<String, usize>,
}

impl ElementCounter {
    fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }
    
    fn get_count(&self, name: &str) -> usize {
        self.counts.get(name).copied().unwrap_or(0)
    }
}

impl<'a> Visitor<'a> for ElementCounter {
    fn visit_element_enter(&mut self, element: &mut Element<'a>) -> Result<(), VexyError> {
        let count = self.counts.entry(element.name.to_string()).or_insert(0);
        *count += 1;
        Ok(())
    }
}

// Test visitor that collects attributes
struct AttributeCollector {
    attributes: Vec<(String, String)>,
}

impl AttributeCollector {
    fn new() -> Self {
        Self {
            attributes: Vec::new(),
        }
    }
}

impl<'a> Visitor<'a> for AttributeCollector {
    fn visit_element_enter(&mut self, element: &mut Element<'a>) -> Result<(), VexyError> {
        for (key, value) in &element.attributes {
            self.attributes.push((key.to_string(), value.to_string()));
        }
        Ok(())
    }
}

// Test visitor that modifies elements
struct ElementModifier {
    target_name: String,
    new_attribute: (String, String),
}

impl ElementModifier {
    fn new(target_name: String, attr_name: String, attr_value: String) -> Self {
        Self {
            target_name,
            new_attribute: (attr_name, attr_value),
        }
    }
}

impl<'a> Visitor<'a> for ElementModifier {
    fn visit_element_enter(&mut self, element: &mut Element<'a>) -> Result<(), VexyError> {
        if element.name == self.target_name {
            element.set_attr(
                self.new_attribute.0.clone(),
                self.new_attribute.1.clone(),
            );
        }
        Ok(())
    }
}

// Test visitor that fails on certain elements
struct FailingVisitor {
    fail_on: String,
}

impl FailingVisitor {
    fn new(fail_on: String) -> Self {
        Self { fail_on }
    }
}

impl<'a> Visitor<'a> for FailingVisitor {
    fn visit_element_enter(&mut self, element: &mut Element<'a>) -> Result<(), VexyError> {
        if element.name == self.fail_on {
            return Err(VexyError::General(format!(
                "Visitor failed on element: {}",
                self.fail_on
            )));
        }
        Ok(())
    }
}

// Test visitor that tracks depth
struct DepthTracker {
    max_depth: usize,
    current_depth: usize,
}

impl DepthTracker {
    fn new() -> Self {
        Self {
            max_depth: 0,
            current_depth: 0,
        }
    }
    
    fn get_max_depth(&self) -> usize {
        self.max_depth
    }
}

impl<'a> Visitor<'a> for DepthTracker {
    fn visit_element_enter(&mut self, _element: &mut Element<'a>) -> Result<(), VexyError> {
        self.current_depth += 1;
        if self.current_depth > self.max_depth {
            self.max_depth = self.current_depth;
        }
        Ok(())
    }
    
    fn visit_element_exit(&mut self, _element: &mut Element<'a>) -> Result<(), VexyError> {
        self.current_depth = self.current_depth.saturating_sub(1);
        Ok(())
    }
}

#[test]
fn test_element_counter_visitor() {
    let svg = r#"<svg>
        <rect width="100" height="100"/>
        <circle cx="50" cy="50" r="25"/>
        <rect x="10" y="10" width="50" height="50"/>
        <g>
            <path d="M 10 10 L 20 20"/>
            <text x="0" y="0">Hello</text>
        </g>
    </svg>"#;
    
    let mut document = parse_svg(svg).unwrap();
    let mut counter = ElementCounter::new();
    
    let result = walk_document(&mut counter, &mut document);
    assert!(result.is_ok());
    
    // Check counts
    assert_eq!(counter.get_count("svg"), 1);
    assert_eq!(counter.get_count("rect"), 2);
    assert_eq!(counter.get_count("circle"), 1);
    assert_eq!(counter.get_count("g"), 1);
    assert_eq!(counter.get_count("path"), 1);
    assert_eq!(counter.get_count("text"), 1);
    assert_eq!(counter.get_count("nonexistent"), 0);
}

#[test]
fn test_attribute_collector_visitor() {
    let svg = r#"<svg width="100" height="100">
        <rect x="10" y="10" width="50" height="50" fill="red"/>
        <circle cx="50" cy="50" r="25" fill="blue"/>
    </svg>"#;
    
    let mut document = parse_svg(svg).unwrap();
    let mut collector = AttributeCollector::new();
    
    let result = walk_document(&mut collector, &mut document);
    assert!(result.is_ok());
    
    // Check that attributes were collected
    assert!(collector.attributes.len() > 0);
    
    // Check for specific attributes
    let has_width = collector.attributes.iter().any(|(k, v)| k == "width" && v == "100");
    let has_height = collector.attributes.iter().any(|(k, v)| k == "height" && v == "100");
    let has_fill_red = collector.attributes.iter().any(|(k, v)| k == "fill" && v == "red");
    let has_fill_blue = collector.attributes.iter().any(|(k, v)| k == "fill" && v == "blue");
    
    assert!(has_width);
    assert!(has_height);
    assert!(has_fill_red);
    assert!(has_fill_blue);
}

#[test]
fn test_element_modifier_visitor() {
    let svg = r#"<svg><rect width="100" height="100"/></svg>"#;
    let mut document = parse_svg(svg).unwrap();
    let mut modifier = ElementModifier::new(
        "rect".to_string(),
        "fill".to_string(),
        "red".to_string(),
    );
    
    let result = walk_document(&mut modifier, &mut document);
    assert!(result.is_ok());
    
    // Check that the attribute was added by finding the rect element
    let rect_element = document.root.children.iter().find_map(|child| {
        if let Node::Element(rect) = child {
            if rect.name == "rect" { Some(rect) } else { None }
        } else { None }
    });
    
    assert!(rect_element.is_some());
    let rect = rect_element.unwrap();
    assert_eq!(rect.attr("fill"), Some("red"));
}

#[test]
fn test_failing_visitor() {
    let svg = r#"<svg><rect/><circle/></svg>"#;
    let mut document = parse_svg(svg).unwrap();
    let mut failing_visitor = FailingVisitor::new("circle".to_string());
    
    let result = walk_document(&mut failing_visitor, &mut document);
    assert!(result.is_err());
    
    // Should be a general error about the failing visitor
    match result.unwrap_err() {
        VexyError::General(msg) => {
            assert!(msg.contains("Visitor failed on element: circle"));
        }
        other => panic!("Expected general error, got: {:?}", other),
    }
}

#[test]
fn test_depth_tracker_visitor() {
    let svg = r#"<svg>
        <g>
            <g>
                <rect/>
            </g>
        </g>
    </svg>"#;
    
    let mut document = parse_svg(svg).unwrap();
    let mut depth_tracker = DepthTracker::new();
    
    let result = walk_document(&mut depth_tracker, &mut document);
    assert!(result.is_ok());
    
    // Should track maximum depth (svg -> g -> g -> rect = 4 levels)
    assert_eq!(depth_tracker.get_max_depth(), 4);
}

#[test]
fn test_walk_element_directly() {
    let svg = r#"<svg><rect width="100" height="100"/></svg>"#;
    let mut document = parse_svg(svg).unwrap();
    
    let mut counter = ElementCounter::new();
    let result = walk_element(&mut counter, &mut document.root);
    assert!(result.is_ok());
    
    // Should count the svg element and its rect child
    assert_eq!(counter.get_count("svg"), 1);
    assert_eq!(counter.get_count("rect"), 1);
}

#[test]
fn test_walk_node_directly() {
    let svg = r#"<svg><rect/></svg>"#;
    let mut document = parse_svg(svg).unwrap();
    
    // Create a node wrapping the root element
    let mut counter = ElementCounter::new();
    
    // Walk the first child of root (which should be rect)
    if !document.root.children.is_empty() {
        let result = walk_node(&mut counter, &mut document.root.children[0]);
        assert!(result.is_ok());
        assert_eq!(counter.get_count("rect"), 1);
    }
}

#[test]
fn test_visitor_with_empty_document() {
    let mut document = Document::new();
    let mut counter = ElementCounter::new();
    
    let result = walk_document(&mut counter, &mut document);
    assert!(result.is_ok());
    
    // Should only count the root svg element
    assert_eq!(counter.get_count("svg"), 1);
}

#[test]
fn test_visitor_with_nested_elements() {
    let svg = r#"<svg>
        <g id="group1">
            <g id="group2">
                <g id="group3">
                    <rect/>
                </g>
            </g>
        </g>
    </svg>"#;
    
    let mut document = parse_svg(svg).unwrap();
    let mut counter = ElementCounter::new();
    
    let result = walk_document(&mut counter, &mut document);
    assert!(result.is_ok());
    
    // Should count all nested elements
    assert_eq!(counter.get_count("svg"), 1);
    assert_eq!(counter.get_count("g"), 3);
    assert_eq!(counter.get_count("rect"), 1);
}

#[test]
fn test_visitor_with_mixed_content() {
    let svg = r#"<svg>
        <!-- comment -->
        <rect/>
        <text>Some text content</text>
        <!-- another comment -->
        <circle/>
    </svg>"#;
    
    let mut document = parse_svg(svg).unwrap();
    let mut counter = ElementCounter::new();
    
    let result = walk_document(&mut counter, &mut document);
    assert!(result.is_ok());
    
    // Should count only elements, not comments or text
    assert_eq!(counter.get_count("svg"), 1);
    assert_eq!(counter.get_count("rect"), 1);
    assert_eq!(counter.get_count("text"), 1);
    assert_eq!(counter.get_count("circle"), 1);
}

#[test]
fn test_visitor_error_propagation() {
    let svg = r#"<svg><rect/><circle/><path/></svg>"#;
    let mut document = parse_svg(svg).unwrap();
    let mut failing_visitor = FailingVisitor::new("circle".to_string());
    
    let result = walk_document(&mut failing_visitor, &mut document);
    assert!(result.is_err());
    
    // Error should propagate up from the visitor
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Visitor failed on element: circle"));
}

#[test]
fn test_visitor_trait_default_implementations() {
    // Test that default implementations work correctly
    struct MinimalVisitor;
    
    impl<'a> Visitor<'a> for MinimalVisitor {
        // All methods use defaults
    }
    
    let svg = r#"<svg><rect/></svg>"#;
    let mut document = parse_svg(svg).unwrap();
    let mut minimal_visitor = MinimalVisitor;
    
    let result = walk_document(&mut minimal_visitor, &mut document);
    assert!(result.is_ok());
}

#[test]
fn test_visitor_with_self_closing_elements() {
    let svg = r#"<svg><rect/><circle/><path/></svg>"#;
    let mut document = parse_svg(svg).unwrap();
    let mut counter = ElementCounter::new();
    
    let result = walk_document(&mut counter, &mut document);
    assert!(result.is_ok());
    
    // Should count all self-closing elements
    assert_eq!(counter.get_count("svg"), 1);
    assert_eq!(counter.get_count("rect"), 1);
    assert_eq!(counter.get_count("circle"), 1);
    assert_eq!(counter.get_count("path"), 1);
}

#[test]
fn test_visitor_performance_with_large_document() {
    // Create a large SVG with many elements
    let mut svg = String::from("<svg>");
    for i in 0..1000 {
        svg.push_str(&format!("<rect id=\"rect{}\"/>", i));
    }
    svg.push_str("</svg>");
    
    let mut document = parse_svg(&svg).unwrap();
    let mut counter = ElementCounter::new();
    
    let result = walk_document(&mut counter, &mut document);
    assert!(result.is_ok());
    
    // Should count all elements efficiently
    assert_eq!(counter.get_count("svg"), 1);
    assert_eq!(counter.get_count("rect"), 1000);
}
