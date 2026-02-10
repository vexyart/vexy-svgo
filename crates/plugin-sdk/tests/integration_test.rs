// this_file: crates/plugin-sdk/tests/integration_test.rs

//! Integration tests for the new plugin architecture

use vexy_svgo_core::ast::{Document, Element, Node};
use vexy_svgo_plugin_sdk::{Plugin, plugins::RemoveCommentsPlugin};

#[test]
fn test_remove_comments_plugin_integration() {
    // Create a test document with comments
    let mut doc = Document::new();

    // Add some children to the root element to test comment removal
    doc.root
        .children
        .push(Node::Comment("Regular comment".to_string().into_boxed_str()));
    doc.root
        .children
        .push(Node::Comment("! Legal comment".to_string().into_boxed_str()));
    doc.root.children.push(Node::Element(Element::new("rect")));

    // Create the plugin
    let plugin = RemoveCommentsPlugin::new();

    // Apply the plugin
    let result = plugin.apply(&mut doc);
    assert!(result.is_ok());

    // Check that comments were processed according to plugin logic
    // The visitor should have removed the regular comment but kept the legal comment
    let comment_count = doc
        .root
        .children
        .iter()
        .filter(|child| matches!(child, Node::Comment(_)))
        .count();

    // With preserve_patterns=true (default), should keep the legal comment
    assert_eq!(comment_count, 1);

    // Verify it's the legal comment that was kept
    if let Some(Node::Comment(comment)) = doc
        .root
        .children
        .iter()
        .find(|child| matches!(child, Node::Comment(_)))
    {
        assert!(comment.starts_with("! Legal"));
    }
}

#[test]
fn test_remove_comments_without_preserve_patterns() {
    let mut doc = Document::new();

    doc.root
        .children
        .push(Node::Comment("Regular comment".to_string().into_boxed_str()));
    doc.root
        .children
        .push(Node::Comment("! Legal comment".to_string().into_boxed_str()));
    doc.root.children.push(Node::Element(Element::new("rect")));

    // Create plugin with preserve_patterns disabled
    let plugin = RemoveCommentsPlugin::with_preserve_patterns(false);

    plugin.apply(&mut doc).unwrap();

    // Should remove all comments
    let comment_count = doc
        .root
        .children
        .iter()
        .filter(|child| matches!(child, Node::Comment(_)))
        .count();

    assert_eq!(comment_count, 0);
}
