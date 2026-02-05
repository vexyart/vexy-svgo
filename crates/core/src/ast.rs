// this_file: crates/core/src/ast.rs

//! Abstract Syntax Tree (AST) for SVG documents
//!
//! This module defines the core data structures for representing SVG documents
//! in memory. The AST is designed for efficient traversal and mutation during
//! optimization passes, with memory optimizations for common use cases.

use indexmap::IndexMap;
use std::borrow::Cow;

/// Represents a complete SVG document, including the root element,
/// surrounding nodes, and metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct Document<'a> {
    /// A list of nodes that appear before the root `<svg>` element,
    /// such as comments or processing instructions.
    pub prologue: Vec<Node<'a>>,
    /// The root `<svg>` element of the document.
    pub root: Element<'a>,
    /// A list of nodes that appear after the root `<svg>` element.
    pub epilogue: Vec<Node<'a>>,
    /// Metadata associated with the document, such as file path and encoding.
    pub metadata: DocumentMetadata,
}

/// Contains metadata about the SVG document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DocumentMetadata {
    /// The original file path of the document, if available.
    pub path: Option<String>,
    /// The character encoding of the document, e.g., "UTF-8".
    pub encoding: Option<String>,
    /// The XML version specified in the document, e.g., "1.0".
    pub version: Option<String>,
}

/// Represents an XML element within the SVG document.
/// Memory optimized with IndexMap for attributes (better cache locality than HashMap)
/// and pre-allocated capacity for common use cases.
#[derive(Debug, Clone, PartialEq)]
pub struct Element<'a> {
    /// The tag name of the element, such as "svg", "path", or "circle".
    ///
    /// Using `Cow` allows for borrowing the name from the input string
    /// when possible, or owning it if it needs to be modified.
    pub name: Cow<'a, str>,
    /// A map of the element's attributes.
    /// Using IndexMap for consistent ordering and better performance.
    pub attributes: IndexMap<Cow<'a, str>, Cow<'a, str>>,
    /// A map of the element's child nodes.
    /// Pre-allocated with capacity for common cases to reduce reallocations.
    pub children: Vec<Node<'a>>,
    /// A map of namespace declarations for this element.
    /// Using IndexMap for consistent ordering.
    pub namespaces: IndexMap<String, String>,
}

/// An enum representing the different types of nodes that can exist
/// in the SVG document tree.
/// 
/// Memory optimization notes:
/// - Using `Box<str>` for strings where possible to reduce memory overhead
/// - Considering compact representations for common node types
#[derive(Debug, Clone, PartialEq)]
pub enum Node<'a> {
    /// An XML element, represented by the `Element` struct.
    Element(Element<'a>),
    /// A text node - using String for now, could be optimized to `Box<str>`
    Text(String),
    /// An XML comment - using String for now, could be optimized to `Box<str>`
    Comment(String),
    /// An XML processing instruction.
    ProcessingInstruction {
        /// The target of the processing instruction.
        target: String,
        /// The data of the processing instruction.
        data: String,
    },
    /// A CDATA section, which is used for escaping blocks of text.
    CData(String),
    /// A DOCTYPE declaration.
    DocType(String),
}

impl<'a> Document<'a> {
    /// Creates a new, empty `Document` with a default `<svg>` root element.
    pub fn new() -> Self {
        Self {
            prologue: Vec::new(),
            root: Element::new("svg"),
            epilogue: Vec::new(),
            metadata: DocumentMetadata::default(),
        }
    }

    /// Returns a reference to the root element of the document.
    pub fn root(&self) -> &Element<'_> {
        &self.root
    }

    /// Returns a mutable reference to the root element of the document.
    pub fn root_mut(&mut self) -> &mut Element<'a> {
        &mut self.root
    }
}

impl<'a> Element<'a> {
    /// Creates a new `Element` with the given tag name.
    /// Pre-allocates reasonable capacity for attributes and children to reduce reallocations.
    pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
        Self {
            name: name.into(),
            // Pre-allocate capacity for a few attributes - most SVG elements have 1-4 attributes
            attributes: IndexMap::with_capacity(4),
            children: Vec::new(),
            // Most elements don't have namespace declarations, so start empty
            namespaces: IndexMap::new(),
        }
    }

    /// Creates a new `Element` with the given tag name and expected number of children.
    /// This allows for better memory allocation when the number of children is known.
    pub fn with_capacity(name: impl Into<Cow<'a, str>>, children_capacity: usize) -> Self {
        Self {
            name: name.into(),
            attributes: IndexMap::with_capacity(4),
            children: Vec::with_capacity(children_capacity),
            namespaces: IndexMap::new(),
        }
    }

    /// Returns the value of the attribute with the given name, if it exists.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attributes.get(name).map(|cow| cow.as_ref())
    }

    /// Sets the value of an attribute. If the attribute already exists,
    /// its value is updated.
    pub fn set_attr(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.attributes.insert(Cow::Owned(name.into()), Cow::Owned(value.into()));
    }

    /// Removes an attribute from the element, returning its value if it existed.
    pub fn remove_attr(&mut self, name: &str) -> Option<String> {
        self.attributes.shift_remove(name).map(|cow| cow.into_owned())
    }

    /// Checks whether the element has an attribute with the given name.
    pub fn has_attr(&self, name: &str) -> bool {
        self.attributes.contains_key(name)
    }

    /// Adds a `Node` to the element's list of children.
    pub fn add_child(&mut self, child: Node<'a>) {
        self.children.push(child);
    }

    /// Removes all children from the element.
    pub fn clear_children(&mut self) {
        self.children.clear();
    }

    /// Returns an iterator over the element's children that are of type `Element`.
    pub fn child_elements(&self) -> impl Iterator<Item = &Element<'_>> {
        self.children.iter().filter_map(|node| {
            if let Node::Element(element) = node {
                Some(element)
            } else {
                None
            }
        })
    }

    /// Returns a mutable iterator over the element's children that are elements.
    pub fn child_elements_mut(&mut self) -> impl Iterator<Item = &mut Element<'a>> {
        self.children.iter_mut().filter_map(move |node| {
            if let Node::Element(element) = node {
                Some(element)
            } else {
                None
            }
        })
    }

    /// Checks if the element has any children.
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Checks if the element contains only whitespace text nodes and comments.
    pub fn is_whitespace_only(&self) -> bool {
        self.children.iter().all(|child| match child {
            Node::Text(text) => text.trim().is_empty(),
            Node::Comment(_) => true,
            _ => false,
        })
    }

    /// Estimates the memory usage of this element and its children (in bytes).
    /// This is useful for memory profiling and optimization.
    pub fn estimated_memory_usage(&self) -> usize {
        let mut usage = std::mem::size_of::<Self>();
        
        // Add name size
        usage += self.name.len();
        
        // Add attributes
        for (key, value) in &self.attributes {
            usage += std::mem::size_of::<String>() * 2 + key.len() + value.len();
        }
        
        // Add namespaces
        for (key, value) in &self.namespaces {
            usage += std::mem::size_of::<String>() * 2 + key.len() + value.len();
        }
        
        // Add children recursively
        for child in &self.children {
            usage += child.estimated_memory_usage();
        }
        
        usage
    }

    /// Optimizes the memory usage of this element by shrinking collections to fit.
    /// Call this after all modifications are complete to reduce memory overhead.
    pub fn optimize_memory(&mut self) {
        self.attributes.shrink_to_fit();
        self.namespaces.shrink_to_fit();
        self.children.shrink_to_fit();
        
        // Recursively optimize children
        for child in &mut self.children {
            if let Node::Element(element) = child {
                element.optimize_memory();
            }
        }
    }
}

impl<'a> Node<'a> {
    /// Returns `true` if the node is an `Element`.
    pub fn is_element(&self) -> bool {
        matches!(self, Node::Element(_))
    }

    /// Returns `true` if the node is a `Text` node.
    pub fn is_text(&self) -> bool {
        matches!(self, Node::Text(_))
    }

    /// Returns `true` if the node is a `Comment`.
    pub fn is_comment(&self) -> bool {
        matches!(self, Node::Comment(_))
    }

    /// Returns `true` if the node is a `DocType` declaration.
    pub fn is_doctype(&self) -> bool {
        matches!(self, Node::DocType(_))
    }

    /// Returns a reference to the `Element` if the node is an element,
    /// otherwise returns `None`.
    pub fn as_element(&self) -> Option<&Element<'_>> {
        if let Node::Element(element) = self {
            Some(element)
        } else {
            None
        }
    }

    /// Returns a mutable reference to the `Element` if the node is an element,
    /// otherwise returns `None`.
    pub fn as_element_mut(&mut self) -> Option<&mut Element<'a>> {
        if let Node::Element(element) = self {
            Some(element)
        } else {
            None
        }
    }

    /// Returns a reference to the `String` if the node is a text node,
    /// otherwise returns `None`.
    pub fn as_text(&self) -> Option<&String> {
        if let Node::Text(text) = self {
            Some(text)
        } else {
            None
        }
    }

    /// Estimates the memory usage of this node (in bytes).
    pub fn estimated_memory_usage(&self) -> usize {
        match self {
            Node::Element(element) => element.estimated_memory_usage(),
            Node::Text(text) => std::mem::size_of::<String>() + text.len(),
            Node::Comment(comment) => std::mem::size_of::<String>() + comment.len(),
            Node::ProcessingInstruction { target, data } => {
                std::mem::size_of::<String>() * 2 + target.len() + data.len()
            }
            Node::CData(cdata) => std::mem::size_of::<String>() + cdata.len(),
            Node::DocType(doctype) => std::mem::size_of::<String>() + doctype.len(),
        }
    }
}

impl<'a> Default for Document<'a> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_creation() {
        let element = Element::new("rect");
        assert_eq!(element.name, "rect");
        assert!(element.attributes.is_empty());
        assert!(element.children.is_empty());
    }

    #[test]
    fn test_element_with_capacity() {
        let element = Element::with_capacity("g", 10);
        assert_eq!(element.name, "g");
        assert_eq!(element.children.capacity(), 10);
    }

    #[test]
    fn test_attribute_operations() {
        let mut element = Element::new("rect");

        element.set_attr("x".to_string(), "10".to_string());
        assert_eq!(element.attr("x"), Some("10"));
        assert!(element.has_attr("x"));

        let removed = element.remove_attr("x");
        assert_eq!(removed, Some("10".to_string()));
        assert!(!element.has_attr("x"));
    }

    #[test]
    fn test_child_operations() {
        let mut parent = Element::new("g");
        let child = Element::new("rect");

        parent.add_child(Node::Element(child));
        parent.add_child(Node::Text("test".to_string()));
        parent.add_child(Node::Comment("comment".to_string()));

        assert_eq!(parent.children.len(), 3);
        assert_eq!(parent.child_elements().count(), 1);
        assert!(!parent.is_empty());
    }

    #[test]
    fn test_whitespace_detection() {
        let mut element = Element::new("g");
        element.add_child(Node::Text("   \n  ".to_string()));
        element.add_child(Node::Comment("comment".to_string()));

        assert!(element.is_whitespace_only());

        element.add_child(Node::Text("content".to_string()));
        assert!(!element.is_whitespace_only());
    }

    #[test]
    fn test_memory_estimation() {
        let mut element = Element::new("rect");
        element.set_attr("x", "10");
        element.set_attr("y", "20");
        element.add_child(Node::Text("content".to_string()));

        let usage = element.estimated_memory_usage();
        assert!(usage > 0);
        
        // Should include element size, attribute sizes, and child sizes
        let base_size = std::mem::size_of::<Element>();
        assert!(usage >= base_size);
    }

    #[test]
    fn test_memory_optimization() {
        let mut element = Element::with_capacity("g", 100);
        
        // Add only a few children, leaving most capacity unused
        element.add_child(Node::Text("test".to_string()));
        element.add_child(Node::Comment("comment".to_string()));
        
        // Before optimization, capacity should be 100
        assert_eq!(element.children.capacity(), 100);
        
        // After optimization, capacity should be reduced to fit
        element.optimize_memory();
        assert_eq!(element.children.capacity(), 2);
    }

    #[test]
    fn test_node_memory_usage() {
        let text_node = Node::Text("Hello, World!".to_string());
        let comment_node = Node::Comment("This is a comment".to_string());
        let pi_node = Node::ProcessingInstruction {
            target: "xml".to_string(),
            data: "version=\"1.0\"".to_string(),
        };

        assert!(text_node.estimated_memory_usage() > 0);
        assert!(comment_node.estimated_memory_usage() > 0);
        assert!(pi_node.estimated_memory_usage() > 0);
    }
}