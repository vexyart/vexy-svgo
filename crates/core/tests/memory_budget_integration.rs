// this_file: crates/core/tests/memory_budget_integration.rs

use std::sync::Arc;

use vexy_svgo_core::{optimize, Config, Document, Element, MemoryBudget, Node, OptimizeOptions};

#[test]
fn test_document_with_memory_budget() {
    let budget = Arc::new(MemoryBudget::new(1024));
    let doc = Document::new().with_memory_budget(budget.clone());

    assert!(doc.memory_budget().is_some());
    assert_eq!(budget.used(), 0);
}

#[test]
fn test_element_add_child_checked_success() {
    let budget = Arc::new(MemoryBudget::new(10_000));
    let mut elem = Element::new("svg");

    let child = Node::Text("Hello".to_string().into_boxed_str());
    let result = elem.add_child_checked(child, Some(&budget));

    assert!(result.is_ok());
    assert_eq!(elem.children.len(), 1);
    assert!(budget.used() > 0);
}

#[test]
fn test_element_add_child_checked_budget_exceeded() {
    let budget = Arc::new(MemoryBudget::new(10));
    let mut elem = Element::new("svg");

    let child = Node::Text("Hello, World!".to_string().into_boxed_str());
    let result = elem.add_child_checked(child, Some(&budget));

    assert!(result.is_err());
    assert_eq!(elem.children.len(), 0);
}

#[test]
fn test_element_set_attr_checked_success() {
    let budget = Arc::new(MemoryBudget::new(1000));
    let mut elem = Element::new("rect");

    let result = elem.set_attr_checked("width", "100", Some(&budget));

    assert!(result.is_ok());
    assert_eq!(elem.attr("width"), Some("100"));
    assert!(budget.used() > 0);
}

#[test]
fn test_element_set_attr_checked_budget_exceeded() {
    let budget = Arc::new(MemoryBudget::new(10));
    let mut elem = Element::new("rect");

    let result = elem.set_attr_checked("width", "100", Some(&budget));

    assert!(result.is_err());
    assert!(!elem.has_attr("width"));
}

#[test]
fn test_optimize_with_memory_budget() {
    let svg = r#"<svg><rect width="100" height="100"/></svg>"#;
    let budget = Arc::new(MemoryBudget::new(100 * 1024 * 1024));

    let options = OptimizeOptions::new(Config::default()).with_memory_budget(budget.clone());

    let result = optimize(svg, options);
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(result.data.contains("<svg"));
}

#[test]
fn test_optimize_with_very_small_budget() {
    let svg = r#"<svg><rect width="100" height="100"/></svg>"#;
    let budget = Arc::new(MemoryBudget::new(10));

    let options = OptimizeOptions::new(Config::default()).with_memory_budget(budget.clone());

    let result = optimize(svg, options);
    assert!(result.is_ok());
}

#[test]
fn test_streaming_config_create_memory_budget() {
    use vexy_svgo_core::StreamingConfig;

    let config = StreamingConfig::default();
    let budget = config.create_memory_budget();

    assert!(budget.is_some());
    let budget = budget.unwrap();
    assert_eq!(budget.limit(), 100 * 1024 * 1024);
}

#[test]
fn test_memory_budget_without_limit() {
    let mut elem = Element::new("svg");

    let child = Node::Text("Hello".to_string().into_boxed_str());
    let result = elem.add_child_checked(child, None);

    assert!(result.is_ok());
    assert_eq!(elem.children.len(), 1);
}
