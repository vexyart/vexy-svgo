// this_file: crates/core/src/optimizer/parallel.rs

//! Parallel optimization support for large SVG files
//!
//! This module provides multi-threading capabilities for optimizing large SVG files
//! by processing independent elements in parallel using rayon.

use std::sync::Arc;

use rayon::prelude::*;

use crate::ast::{Document, Element, Node};

/// Configuration for parallel optimization
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Minimum file size in bytes to enable parallel processing
    pub size_threshold:    usize,
    /// Minimum number of elements to enable parallel processing
    pub element_threshold: usize,
    /// Number of threads to use (0 = use rayon default)
    pub num_threads:       usize,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            size_threshold:    1024 * 1024, // 1MB
            element_threshold: 1000,
            num_threads:       0, // Use rayon default
        }
    }
}

/// Result of parallel optimization with statistics
pub struct ParallelOptimizationResult<'a> {
    /// The optimized document
    pub document:           Document<'a>,
    /// Number of elements processed
    pub elements_processed: usize,
    /// Number of threads used
    pub threads_used:       usize,
    /// Time taken for optimization
    pub duration:           std::time::Duration,
}

/// Check if a document should be processed in parallel
pub fn should_parallelize(document: &Document, estimated_size: usize, config: &ParallelConfig) -> bool {
    if estimated_size < config.size_threshold {
        return false;
    }

    let element_count = count_elements(&document.root);
    element_count >= config.element_threshold
}

/// Configure rayon thread pool
pub fn configure_thread_pool(config: &ParallelConfig) {
    if config.num_threads > 0 {
        rayon::ThreadPoolBuilder::new().num_threads(config.num_threads).build_global().ok();
    }
}

/// Process groups of independent elements in parallel
///
/// This function identifies groups of elements that can be safely processed
/// in parallel (no cross-references) and processes them concurrently.
pub fn process_independent_groups<'a>(
    element: &mut Element<'a>,
    process_fn: Arc<dyn Fn(&mut Element) + Send + Sync>,
) -> usize {
    // First, identify independent groups
    let groups = identify_independent_groups(element);
    let mut processed_count = 0;

    for group in groups {
        if group.len() > 10 {
            // Process large groups in parallel
            processed_count += process_group_parallel(element, &group, process_fn.clone());
        } else {
            // Process small groups sequentially
            for idx in group {
                if let Some(Node::Element(child)) = element.children.get_mut(idx) {
                    process_fn(child);
                    processed_count += 1;
                }
            }
        }
    }

    processed_count
}

/// Process a group of element indices in parallel
fn process_group_parallel<'a>(
    parent: &mut Element<'a>,
    indices: &[usize],
    process_fn: Arc<dyn Fn(&mut Element) + Send + Sync>,
) -> usize {
    // Extract elements for parallel processing
    let mut elements_to_process: Vec<(usize, Element)> = Vec::new();

    for &idx in indices {
        if let Some(Node::Element(elem)) = parent.children.get(idx) {
            elements_to_process.push((idx, elem.clone()));
        }
    }

    // Process in parallel
    let processed_elements: Vec<(usize, Element)> = elements_to_process
        .into_par_iter()
        .map(|(idx, mut elem)| {
            process_fn(&mut elem);
            (idx, elem)
        })
        .collect();

    // Update the parent with processed elements
    for (idx, elem) in processed_elements.iter() {
        if let Some(Node::Element(child)) = parent.children.get_mut(*idx) {
            *child = elem.clone();
        }
    }

    processed_elements.len()
}

/// Identify groups of elements that can be processed independently
fn identify_independent_groups(element: &Element) -> Vec<Vec<usize>> {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut current_group: Vec<usize> = Vec::new();
    let mut referenced_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (idx, child) in element.children.iter().enumerate() {
        if let Node::Element(elem) = child {
            let elem_ids = collect_ids(elem);
            let elem_refs = collect_references(elem);

            // Check if this element can be added to current group
            let mut can_add = true;

            // Check if element references any IDs from current group
            for ref_id in &elem_refs {
                if referenced_ids.contains(ref_id) {
                    can_add = false;
                    break;
                }
            }

            if can_add {
                // Add to current group
                current_group.push(idx);
                referenced_ids.extend(elem_ids);
            } else {
                // Start new group
                if !current_group.is_empty() {
                    groups.push(current_group);
                }
                current_group = vec![idx];
                referenced_ids = elem_ids;
            }
        }
    }

    if !current_group.is_empty() {
        groups.push(current_group);
    }

    groups
}

/// Count total elements in a tree
pub fn count_elements(element: &Element) -> usize {
    let mut count = 1;
    for child in &element.children {
        if let Node::Element(elem) = child {
            count += count_elements(elem);
        }
    }
    count
}

/// Collect all IDs from an element tree
fn collect_ids(element: &Element) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();

    if let Some(id) = element.attr("id") {
        ids.insert(id.to_string());
    }

    for child in &element.children {
        if let Node::Element(elem) = child {
            ids.extend(collect_ids(elem));
        }
    }

    ids
}

/// Collect all ID references from an element tree
fn collect_references(element: &Element) -> std::collections::HashSet<String> {
    let mut refs = std::collections::HashSet::new();

    // Check href attributes
    if let Some(href) = element.attr("href") {
        if let Some(stripped) = href.strip_prefix('#') {
            refs.insert(stripped.to_string());
        }
    }

    // Check xlink:href attributes
    if let Some(xlink_href) = element.attr("xlink:href") {
        if let Some(stripped) = xlink_href.strip_prefix('#') {
            refs.insert(stripped.to_string());
        }
    }

    // Check fill/stroke url references
    for attr in [
        "fill",
        "stroke",
        "filter",
        "mask",
        "marker-start",
        "marker-mid",
        "marker-end",
    ] {
        if let Some(value) = element.attr(attr) {
            if value.starts_with("url(#") && value.ends_with(')') {
                let id_ref = &value[5..value.len() - 1];
                refs.insert(id_ref.to_string());
            }
        }
    }

    // Check children recursively
    for child in &element.children {
        if let Node::Element(elem) = child {
            refs.extend(collect_references(elem));
        }
    }

    refs
}

/// Parallel statistics collection for optimization tracking
impl Default for ParallelStats {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ParallelStats {
    pub elements_processed: std::sync::atomic::AtomicUsize,
    pub start_time:         std::time::Instant,
}

impl ParallelStats {
    pub fn new() -> Self {
        Self {
            elements_processed: std::sync::atomic::AtomicUsize::new(0),
            start_time:         std::time::Instant::now(),
        }
    }
}

use std::sync::atomic::AtomicUsize;
use std::time::Instant;

#[derive(Debug)]
pub struct Progress {
    elements_processed: AtomicUsize,
    start_time:         Instant,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            elements_processed: std::sync::atomic::AtomicUsize::new(0),
            start_time:         std::time::Instant::now(),
        }
    }
}

impl Progress {
    pub fn increment(&self, count: usize) {
        self.elements_processed.fetch_add(count, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_processed(&self) -> usize {
        self.elements_processed.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Document;

    #[test]
    fn test_element_counting() {
        let mut doc = Document::new();
        let mut g1 = Element::new("g");
        let mut g2 = Element::new("g");
        let rect1 = Element::new("rect");
        let rect2 = Element::new("rect");

        g2.add_child(Node::Element(rect1));
        g2.add_child(Node::Element(rect2));
        g1.add_child(Node::Element(g2));
        doc.root.add_child(Node::Element(g1));

        let count = count_elements(&doc.root);
        assert_eq!(count, 5); // svg + g + g + rect + rect
    }

    #[test]
    fn test_parallel_config() {
        let config = ParallelConfig::default();
        assert_eq!(config.size_threshold, 1024 * 1024);
        assert_eq!(config.element_threshold, 1000);
        assert_eq!(config.num_threads, 0);
    }

    #[test]
    fn test_id_collection() {
        let mut elem = Element::new("g");
        elem.set_attr("id", "group1");

        let mut child1 = Element::new("rect");
        child1.set_attr("id", "rect1");

        let mut child2 = Element::new("circle");
        child2.set_attr("id", "circle1");

        elem.add_child(Node::Element(child1));
        elem.add_child(Node::Element(child2));

        let ids = collect_ids(&elem);
        assert_eq!(ids.len(), 3);
        assert!(ids.contains("group1"));
        assert!(ids.contains("rect1"));
        assert!(ids.contains("circle1"));
    }

    #[test]
    fn test_reference_collection() {
        let mut elem = Element::new("use");
        elem.set_attr("href", "#rect1");

        let mut child = Element::new("rect");
        child.set_attr("fill", "url(#gradient1)");
        elem.add_child(Node::Element(child));

        let refs = collect_references(&elem);
        assert_eq!(refs.len(), 2);
        assert!(refs.contains("rect1"));
        assert!(refs.contains("gradient1"));
    }

    #[test]
    fn test_independent_groups() {
        let mut parent = Element::new("g");

        // Group 1: Independent elements
        let rect1 = Element::new("rect");
        let rect2 = Element::new("rect");

        // Group 2: Element with reference
        let mut use_elem = Element::new("use");
        use_elem.set_attr("href", "#rect1");

        parent.add_child(Node::Element(rect1));
        parent.add_child(Node::Element(rect2));
        parent.add_child(Node::Element(use_elem));

        let groups = identify_independent_groups(&parent);
        assert!(!groups.is_empty());
    }
}
