// this_file: crates/plugin-sdk/examples/plugin_composition.rs

//! Plugin composition example
//!
//! This example demonstrates how to use multiple plugins together
//! with the new plugin registry system.

use serde_json::json;
use std::borrow::Cow;
use vexy_svgo_core::ast::{Document, Element, Node};
use vexy_svgo_core::{PluginConfig, PluginRegistry};
use vexy_svgo_plugin_sdk::{Plugin, plugins::{RemoveCommentsPlugin, RemoveEmptyAttrsPlugin}};

fn main() -> anyhow::Result<()> {
    println!("Plugin Composition Example");
    println!("==========================");

    // Create a test document with various elements to optimize
    let mut doc = create_test_document();

    println!("Original document state:");
    print_document_stats(&doc);

    // Create plugin registry and register plugins
    let mut registry = PluginRegistry::new();
    registry.register("removeComments", || RemoveCommentsPlugin::new());
    registry.register("removeEmptyAttrs", || RemoveEmptyAttrsPlugin::new());

    // Configure plugins
    let plugin_configs = vec![
        PluginConfig::WithParams {
            name: "removeComments".to_string(),
            params: json!({"preservePatterns": true}),
        },
        PluginConfig::WithParams {
            name: "removeEmptyAttrs".to_string(),
            params: json!({"preserveClass": false, "preserveId": false}),
        },
    ];

    // Apply plugins in sequence
    println!("\nApplying plugins...");
    registry.apply_plugins(&mut doc, &plugin_configs)?;

    println!("Optimized document state:");
    print_document_stats(&doc);

    // Demonstrate individual plugin application
    println!("\n--- Individual Plugin Demonstration ---");
    demonstrate_individual_plugins()?;

    Ok(())
}

fn create_test_document() -> Document<'static> {
    let mut doc = Document::new();

    // Add various types of content to test plugins
    doc.root.name = Cow::Borrowed("svg");
    doc.root
        .attributes
        .insert(Cow::Borrowed("width"), Cow::Borrowed("100"));
    doc.root
        .attributes
        .insert(Cow::Borrowed("height"), Cow::Borrowed("100"));
    doc.root
        .attributes
        .insert(Cow::Borrowed("fill"), Cow::Borrowed("")); // Empty attribute
    doc.root
        .attributes
        .insert(Cow::Borrowed("stroke"), Cow::Borrowed("  ")); // Whitespace-only
    doc.root
        .attributes
        .insert(Cow::Borrowed("class"), Cow::Borrowed("")); // Empty class

    // Add children with comments and empty attributes
    doc.root
        .children
        .push(Node::Comment("Regular comment to be removed".to_string().into_boxed_str()));
    doc.root
        .children
        .push(Node::Comment("! Legal comment to be preserved".to_string().into_boxed_str()));

    let mut rect = Element::new("rect");
    rect.attributes.insert(Cow::Borrowed("x"), Cow::Borrowed("10"));
    rect.attributes.insert(Cow::Borrowed("y"), Cow::Borrowed("10"));
    rect.attributes
        .insert(Cow::Borrowed("width"), Cow::Borrowed("50"));
    rect.attributes
        .insert(Cow::Borrowed("height"), Cow::Borrowed("50"));
    rect.attributes
        .insert(Cow::Borrowed("fill"), Cow::Borrowed("blue"));
    rect.attributes.insert(Cow::Borrowed("stroke"), Cow::Borrowed("")); // Empty
    rect.attributes
        .insert(Cow::Borrowed("opacity"), Cow::Borrowed("")); // Empty

    doc.root.children.push(Node::Element(rect));
    doc.root
        .children
        .push(Node::Comment("Another comment".to_string().into_boxed_str()));

    doc
}

fn print_document_stats(doc: &Document) {
    let comment_count = count_comments(&doc.root);
    let empty_attr_count = count_empty_attributes(&doc.root);
    let total_attr_count = count_total_attributes(&doc.root);

    println!("  Comments: {}", comment_count);
    println!("  Empty attributes: {}", empty_attr_count);
    println!("  Total attributes: {}", total_attr_count);
}

fn count_comments(element: &Element) -> usize {
    let mut count = 0;
    for child in &element.children {
        match child {
            Node::Comment(_) => count += 1,
            Node::Element(elem) => count += count_comments(elem),
            _ => {}
        }
    }
    count
}

fn count_empty_attributes(element: &Element) -> usize {
    let mut count = element
        .attributes
        .values()
        .filter(|v| v.trim().is_empty())
        .count();

    for child in &element.children {
        if let Node::Element(elem) = child {
            count += count_empty_attributes(elem);
        }
    }
    count
}

fn count_total_attributes(element: &Element) -> usize {
    let mut count = element.attributes.len();
    for child in &element.children {
        if let Node::Element(elem) = child {
            count += count_total_attributes(elem);
        }
    }
    count
}

fn demonstrate_individual_plugins() -> anyhow::Result<()> {
    println!("Testing RemoveCommentsPlugin in isolation...");
    let mut doc1 = create_test_document();
    let comment_plugin = RemoveCommentsPlugin::new();
    comment_plugin.apply(&mut doc1)?;
    print_document_stats(&doc1);

    println!("\nTesting RemoveEmptyAttrsPlugin in isolation...");
    let mut doc2 = create_test_document();
    let attrs_plugin = RemoveEmptyAttrsPlugin::new();
    attrs_plugin.apply(&mut doc2)?;
    print_document_stats(&doc2);

    println!("\nTesting plugins with different configurations...");
    let mut doc3 = create_test_document();
    let comment_plugin_no_preserve = RemoveCommentsPlugin::with_preserve_patterns(false);
    comment_plugin_no_preserve.apply(&mut doc3)?;
    print_document_stats(&doc3);

    Ok(())
}
