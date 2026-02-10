// this_file: crates/core/src/optimizer/memory.rs

//! Memory budgeting system for optimizer runtime
//!
//! Provides approximate memory tracking during optimization to prevent
//! excessive memory usage on large or malicious SVG files.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct MemoryBudget {
    limit: usize,
    used:  Arc<AtomicUsize>,
}

impl PartialEq for MemoryBudget {
    fn eq(&self, other: &Self) -> bool {
        self.limit == other.limit && self.used() == other.used()
    }
}

impl MemoryBudget {
    /// Create a new memory budget with the specified limit
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum memory usage in bytes
    ///
    /// # Examples
    ///
    /// ```rust
    /// use vexy_svgo_core::optimizer::memory::MemoryBudget;
    ///
    /// // 100MB limit
    /// let budget = MemoryBudget::new(100 * 1024 * 1024);
    /// ```
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            used: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Attempt to allocate memory from the budget
    ///
    /// # Arguments
    ///
    /// * `size` - Number of bytes to allocate
    ///
    /// # Returns
    ///
    /// `Ok(())` if allocation succeeds, `Err(MemoryError)` if budget exceeded
    ///
    /// # Examples
    ///
    /// ```rust
    /// use vexy_svgo_core::optimizer::memory::MemoryBudget;
    ///
    /// let budget = MemoryBudget::new(1024);
    /// assert!(budget.allocate(512).is_ok());
    /// assert!(budget.allocate(512).is_ok());
    /// assert!(budget.allocate(512).is_err()); // Exceeds budget
    /// ```
    pub fn allocate(&self, size: usize) -> Result<(), MemoryError> {
        let current = self.used.fetch_add(size, Ordering::Relaxed);
        let new_total = current + size;

        if new_total > self.limit {
            // Rollback the allocation
            self.used.fetch_sub(size, Ordering::Relaxed);
            Err(MemoryError::BudgetExceeded {
                requested: size,
                used:      current,
                limit:     self.limit,
            })
        } else {
            Ok(())
        }
    }

    /// Deallocate memory from the budget
    ///
    /// # Arguments
    ///
    /// * `size` - Number of bytes to deallocate
    ///
    /// # Examples
    ///
    /// ```rust
    /// use vexy_svgo_core::optimizer::memory::MemoryBudget;
    ///
    /// let budget = MemoryBudget::new(1024);
    /// budget.allocate(512).unwrap();
    /// budget.deallocate(512);
    /// assert_eq!(budget.used(), 0);
    /// ```
    pub fn deallocate(&self, size: usize) {
        self.used.fetch_sub(size, Ordering::Relaxed);
    }

    /// Get current memory usage
    ///
    /// # Returns
    ///
    /// Current approximate memory usage in bytes
    pub fn used(&self) -> usize {
        self.used.load(Ordering::Relaxed)
    }

    /// Get memory limit
    ///
    /// # Returns
    ///
    /// Maximum memory limit in bytes
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Get remaining memory budget
    ///
    /// # Returns
    ///
    /// Remaining memory available in bytes
    pub fn remaining(&self) -> usize {
        let current = self.used();
        self.limit.saturating_sub(current)
    }

    /// Check if budget is exceeded
    ///
    /// # Returns
    ///
    /// `true` if current usage exceeds limit
    pub fn is_exceeded(&self) -> bool {
        self.used() > self.limit
    }

    /// Reset the budget to zero usage
    pub fn reset(&self) {
        self.used.store(0, Ordering::Relaxed);
    }
}

/// Memory budget error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum MemoryError {
    /// Memory budget exceeded
    #[error("Memory budget exceeded: requested {requested} bytes, used {used}/{limit} bytes")]
    BudgetExceeded {
        /// Bytes requested
        requested: usize,
        /// Bytes already used
        used:      usize,
        /// Budget limit
        limit:     usize,
    },
}

/// Size estimation heuristics for SVG nodes
pub mod estimator {
    use crate::ast::{Element, Node};

    /// Estimate size of an element (base overhead + attributes + children)
    ///
    /// Heuristic: 200 bytes base + 50 bytes per attribute + key/value lengths
    pub fn estimate_element(elem: &Element<'_>) -> usize {
        let mut size = 200; // Base element overhead

        // Attributes: 50 bytes overhead + key + value
        for (key, value) in &elem.attributes {
            size += 50 + key.len() + value.len();
        }

        // Namespaces
        for (key, value) in &elem.namespaces {
            size += 50 + key.len() + value.len();
        }

        // Tag name
        size += elem.name.len();

        size
    }

    /// Estimate size of a node
    pub fn estimate_node(node: &Node<'_>) -> usize {
        match node {
            Node::Element(elem) => estimate_element(elem),
            Node::Text(text) => 40 + text.len(),
            Node::Comment(comment) => 40 + comment.len(),
            Node::ProcessingInstruction { target, data } => 40 + target.len() + data.len(),
            Node::CData(data) => 40 + data.len(),
            Node::DocType(doctype) => 40 + doctype.len(),
        }
    }

    /// Estimate size for adding a child to an element
    pub fn estimate_child_addition(child: &Node<'_>) -> usize {
        // Child node + Vec growth overhead
        estimate_node(child) + 32
    }

    /// Estimate size for adding an attribute
    pub fn estimate_attribute_addition(key: &str, value: &str) -> usize {
        50 + key.len() + value.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_creation() {
        let budget = MemoryBudget::new(1024);
        assert_eq!(budget.limit(), 1024);
        assert_eq!(budget.used(), 0);
        assert_eq!(budget.remaining(), 1024);
        assert!(!budget.is_exceeded());
    }

    #[test]
    fn test_budget_allocation() {
        let budget = MemoryBudget::new(1000);

        assert!(budget.allocate(400).is_ok());
        assert_eq!(budget.used(), 400);
        assert_eq!(budget.remaining(), 600);

        assert!(budget.allocate(400).is_ok());
        assert_eq!(budget.used(), 800);
        assert_eq!(budget.remaining(), 200);

        // This should fail - exceeds budget
        assert!(budget.allocate(400).is_err());
        // Usage should not change after failed allocation
        assert_eq!(budget.used(), 800);
    }

    #[test]
    fn test_budget_deallocation() {
        let budget = MemoryBudget::new(1000);

        budget.allocate(500).unwrap();
        assert_eq!(budget.used(), 500);

        budget.deallocate(200);
        assert_eq!(budget.used(), 300);
        assert_eq!(budget.remaining(), 700);
    }

    #[test]
    fn test_budget_exceeded() {
        let budget = MemoryBudget::new(100);

        budget.allocate(50).unwrap();
        assert!(!budget.is_exceeded());

        budget.allocate(50).unwrap();
        assert!(!budget.is_exceeded());

        // Try to exceed
        let result = budget.allocate(50);
        assert!(result.is_err());

        if let Err(MemoryError::BudgetExceeded { requested, used, limit }) = result {
            assert_eq!(requested, 50);
            assert_eq!(used, 100);
            assert_eq!(limit, 100);
        }
    }

    #[test]
    fn test_budget_reset() {
        let budget = MemoryBudget::new(1000);

        budget.allocate(500).unwrap();
        assert_eq!(budget.used(), 500);

        budget.reset();
        assert_eq!(budget.used(), 0);
        assert_eq!(budget.remaining(), 1000);
    }

    #[test]
    fn test_estimator_node() {
        use crate::ast::Node;

        let text = Node::Text("Hello, World!".to_string().into_boxed_str());
        let size = estimator::estimate_node(&text);
        assert_eq!(size, 40 + 13); // 40 base + text length

        let comment = Node::Comment("Test comment".to_string().into_boxed_str());
        let size = estimator::estimate_node(&comment);
        assert_eq!(size, 40 + 12);
    }

    #[test]
    fn test_estimator_attribute() {
        let size = estimator::estimate_attribute_addition("width", "100");
        assert_eq!(size, 50 + 5 + 3); // 50 base + key len + value len
    }
}
