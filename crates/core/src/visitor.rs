// this_file: crates/core/src/visitor.rs

//! Visitor pattern implementation for SVG AST traversal
//!
//! This module provides a generic visitor pattern for traversing and modifying
//! SVG documents. Plugins can implement the Visitor trait to define their
//! transformation logic in a structured way.

use crate::ast::{Document, Element, Node};
use crate::error::VexyError;

/// Visitor trait for AST traversal
///
/// Implement this trait to define how to traverse and modify the SVG document structure.
/// The visitor pattern provides hooks for entering and exiting elements, as well as
/// processing different node types.
pub trait Visitor<'a> {
    /// Called when entering an element (before processing children)
    fn visit_element_enter(&mut self, element: &mut Element<'a>) -> Result<(), VexyError> {
        let _ = element;
        Ok(())
    }

    /// Called when exiting an element (after processing children)
    fn visit_element_exit(&mut self, element: &mut Element<'a>) -> Result<(), VexyError> {
        let _ = element;
        Ok(())
    }

    /// Called when visiting text content
    fn visit_text(&mut self, text: &str) -> Result<(), VexyError> {
        let _ = text;
        Ok(())
    }

    /// Called when visiting a comment
    fn visit_comment(&mut self, comment: &str) -> Result<(), VexyError> {
        let _ = comment;
        Ok(())
    }

    /// Called when visiting a processing instruction
    fn visit_processing_instruction(&mut self, target: &str, data: &str) -> Result<(), VexyError> {
        let _ = (target, data);
        Ok(())
    }

    /// Called when visiting CDATA
    fn visit_cdata(&mut self, cdata: &str) -> Result<(), VexyError> {
        let _ = cdata;
        Ok(())
    }

    /// Called when visiting DOCTYPE
    fn visit_doctype(&mut self, doctype: &str) -> Result<(), VexyError> {
        let _ = doctype;
        Ok(())
    }

    /// Called when visiting the document root
    fn visit_document(&mut self, document: &mut Document<'a>) -> Result<(), VexyError>
    where
        Self: Sized,
    {
        walk_document(self, document)
    }
}

/// Walk through a document and call visitor methods
pub fn walk_document<'a, V: Visitor<'a> + ?Sized>(
    visitor: &mut V,
    document: &mut Document<'a>,
) -> Result<(), VexyError> {
    // Visit prologue nodes
    for node in &mut document.prologue {
        walk_node(visitor, node)?;
    }

    // Visit root element
    walk_element(visitor, &mut document.root)?;

    // Visit epilogue nodes
    for node in &mut document.epilogue {
        walk_node(visitor, node)?;
    }

    Ok(())
}

/// Walk through an element and call visitor methods
pub fn walk_element<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, element: &mut Element<'a>) -> Result<(), VexyError> {
    // Enter element
    visitor.visit_element_enter(element)?;

    // Visit children (collect indices to avoid borrow issues)
    let child_count = element.children.len();
    for i in 0..child_count {
        walk_node(visitor, &mut element.children[i])?;
    }

    // Exit element
    visitor.visit_element_exit(element)?;

    Ok(())
}

/// Walk through a node and call appropriate visitor methods
pub fn walk_node<'a, V: Visitor<'a> + ?Sized>(visitor: &mut V, node: &mut Node<'a>) -> Result<(), VexyError> {
    match node {
        Node::Element(element) => walk_element(visitor, element),
        Node::Text(text) => visitor.visit_text(text),
        Node::Comment(comment) => visitor.visit_comment(comment),
        Node::ProcessingInstruction { target, data } => visitor.visit_processing_instruction(target, data),
        Node::CData(cdata) => visitor.visit_cdata(cdata),
        Node::DocType(doctype) => visitor.visit_doctype(doctype),
    }
}

/// Utility trait for filtering elements during traversal
pub trait ElementFilter {
    /// Return true if the element should be visited
    fn should_visit<'a>(&self, element: &Element<'a>) -> bool;
}

/// Filter that matches elements by tag name
#[derive(Debug, Clone)]
pub struct TagNameFilter {
    pub tag_names: Vec<String>,
}

impl TagNameFilter {
    pub fn new(tag_names: Vec<&str>) -> Self {
        Self {
            tag_names: tag_names.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn single(tag_name: &str) -> Self {
        Self {
            tag_names: vec![tag_name.to_string()],
        }
    }
}

impl ElementFilter for TagNameFilter {
    fn should_visit<'a>(&self, element: &Element<'a>) -> bool {
        self.tag_names.iter().any(|name| element.name == name.as_str())
    }
}

/// Filter that matches elements by attribute presence
#[derive(Debug, Clone)]
pub struct AttributeFilter {
    pub attribute_name: String,
    pub required_value: Option<String>,
}

impl AttributeFilter {
    pub fn new(attribute_name: &str) -> Self {
        Self {
            attribute_name: attribute_name.to_string(),
            required_value: None,
        }
    }

    pub fn with_value(attribute_name: &str, value: &str) -> Self {
        Self {
            attribute_name: attribute_name.to_string(),
            required_value: Some(value.to_string()),
        }
    }
}

impl ElementFilter for AttributeFilter {
    fn should_visit<'a>(&self, element: &Element<'a>) -> bool {
        match element.attributes.get(self.attribute_name.as_str()) {
            Some(value) => match &self.required_value {
                Some(required) => value == required,
                None => true,
            },
            None => false,
        }
    }
}

/// Filtered visitor that only visits elements matching a filter
pub struct FilteredVisitor<V, F: ElementFilter> {
    visitor: V,
    filter:  F,
}

impl<V, F: ElementFilter> FilteredVisitor<V, F> {
    pub fn new(visitor: V, filter: F) -> Self {
        Self { visitor, filter }
    }
}

impl<'a, V: Visitor<'a>, F: ElementFilter> Visitor<'a> for FilteredVisitor<V, F> {
    fn visit_element_enter(&mut self, element: &mut Element<'a>) -> Result<(), VexyError> {
        if self.filter.should_visit(element) {
            self.visitor.visit_element_enter(element)
        } else {
            Ok(())
        }
    }

    fn visit_element_exit(&mut self, element: &mut Element<'a>) -> Result<(), VexyError> {
        if self.filter.should_visit(element) {
            self.visitor.visit_element_exit(element)
        } else {
            Ok(())
        }
    }

    fn visit_text(&mut self, text: &str) -> Result<(), VexyError> {
        self.visitor.visit_text(text)
    }

    fn visit_comment(&mut self, comment: &str) -> Result<(), VexyError> {
        self.visitor.visit_comment(comment)
    }

    fn visit_processing_instruction(&mut self, target: &str, data: &str) -> Result<(), VexyError> {
        self.visitor.visit_processing_instruction(target, data)
    }

    fn visit_cdata(&mut self, cdata: &str) -> Result<(), VexyError> {
        self.visitor.visit_cdata(cdata)
    }

    fn visit_doctype(&mut self, doctype: &str) -> Result<(), VexyError> {
        self.visitor.visit_doctype(doctype)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Document, Element, Node};

    struct TestVisitor {
        elements_visited: Vec<String>,
        texts_visited:    Vec<String>,
    }

    impl TestVisitor {
        fn new() -> Self {
            Self {
                elements_visited: Vec::new(),
                texts_visited:    Vec::new(),
            }
        }
    }

    impl<'a> Visitor<'a> for TestVisitor {
        fn visit_element_enter(&mut self, element: &mut Element<'a>) -> Result<(), VexyError> {
            self.elements_visited.push(element.name.to_string());
            Ok(())
        }

        fn visit_text(&mut self, text: &str) -> Result<(), VexyError> {
            self.texts_visited.push(text.to_string());
            Ok(())
        }
    }

    #[test]
    fn test_visitor_traversal() {
        let mut document = Document::new();

        // Create a simple test structure
        let mut rect = Element::new("rect");
        rect.children.push(Node::Text("Hello".into()));

        document.root.children.push(Node::Element(rect));
        document.root.children.push(Node::Text("World".into()));

        let mut visitor = TestVisitor::new();
        visitor.visit_document(&mut document).unwrap();

        assert_eq!(visitor.elements_visited, vec!["svg", "rect"]);
        assert_eq!(visitor.texts_visited, vec!["Hello", "World"]);
    }

    #[test]
    fn test_filtered_visitor() {
        let mut document = Document::new();

        let rect = Element::new("rect");
        let circle = Element::new("circle");

        document.root.children.push(Node::Element(rect));
        document.root.children.push(Node::Element(circle));

        let visitor = TestVisitor::new();
        let filter = TagNameFilter::single("rect");
        let mut filtered_visitor = FilteredVisitor::new(visitor, filter);

        filtered_visitor.visit_document(&mut document).unwrap();

        assert_eq!(filtered_visitor.visitor.elements_visited, vec!["rect"]);
    }
}
