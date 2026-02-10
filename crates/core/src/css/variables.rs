// this_file: crates/core/src/css/variables.rs

use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;

use crate::ast::Element;

/// Resolves CSS variables (custom properties) within the SVG document.
///
/// This resolver handles:
/// 1. Inline styles (`style="..."`)
/// 2. Inheritance from parent elements
/// 3. Fallback values (`var(--name, fallback)`)
pub struct CssVariableResolver {
    // Maps element ID/reference to its computed variable scope?
    // Actually, resolving is context-dependent.
}

impl CssVariableResolver {
    /// extract variables from a style string
    fn _parse_inline_style(style: &str) -> HashMap<String, String> {
        static VAR_DECL_RE: OnceLock<Regex> = OnceLock::new();

        let mut vars = HashMap::new();
        for cap in VAR_DECL_RE
            .get_or_init(|| Regex::new(r"(?m)(--[\w-]+)\s*:\s*([^;]+)").unwrap())
            .captures_iter(style)
        {
            if let (Some(name), Some(value)) = (cap.get(1), cap.get(2)) {
                vars.insert(name.as_str().trim().to_string(), value.as_str().trim().to_string());
            }
        }
        vars
    }

    /// Resolve a variable value starting from a given element and walking up the tree.
    /// This requires a traversal context as the AST does not maintain parent links.
    pub fn resolve_value(name: &str, computed_vars: &HashMap<String, String>) -> Option<String> {
        computed_vars.get(name).cloned()
    }
}

/// A context object that tracks CSS variables during document traversal.
#[derive(Debug, Clone, Default)]
pub struct CssScope {
    /// Variables defined at this scope or inherited
    variables: HashMap<String, String>,
}

impl CssScope {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a child scope inheriting from this one, updated with element's inline styles
    pub fn child_scope(&self, element: &Element) -> Self {
        let mut new_scope = self.clone();

        if let Some(style) = element.attr("style") {
            static VAR_DECL_RE: OnceLock<Regex> = OnceLock::new();

            for cap in VAR_DECL_RE.get_or_init(|| Regex::new(r"(--[\w-]+)\s*:\s*([^;]+)").unwrap()).captures_iter(style)
            {
                if let (Some(name), Some(value)) = (cap.get(1), cap.get(2)) {
                    new_scope.variables.insert(name.as_str().trim().to_string(), value.as_str().trim().to_string());
                }
            }
        }

        new_scope
    }

    /// Get the value of a variable in this scope
    pub fn get(&self, name: &str) -> Option<&str> {
        self.variables.get(name).map(|s| s.as_str())
    }

    /// Resolve a value string that might contain var() references
    pub fn resolve(&self, value: &str) -> String {
        static VAR_REF_RE: OnceLock<Regex> = OnceLock::new();

        if !value.contains("var(--") {
            return value.to_string();
        }

        let mut result = value.to_string();

        // Use a loop to handle nested resolutions if needed, or just single pass
        // Extract match info to avoid borrow checker issues
        let match_info = if let Some(caps) = VAR_REF_RE
            .get_or_init(|| Regex::new(r"var\((--[\w-]+)(?:,\s*([^)]+))?\)").unwrap())
            .captures(&result)
        {
            let full_match = caps.get(0).unwrap();
            let var_name = caps.get(1).unwrap().as_str().to_string();
            let fallback = caps.get(2).map(|m| m.as_str().to_string());
            Some((full_match.range(), var_name, fallback))
        } else {
            None
        };

        if let Some((range, var_name, fallback)) = match_info {
            let resolved = self.get(&var_name).map(String::from).or(fallback);

            if let Some(replacement) = resolved {
                result.replace_range(range, &replacement);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Element;

    #[test]
    fn test_scope_inheritance() {
        let _root = Element::new("root"); // No vars
        let root_scope = CssScope::new();

        let mut parent = Element::new("g");
        parent.set_attr("style", "--color: red; --size: 10px;");
        let parent_scope = root_scope.child_scope(&parent);

        assert_eq!(parent_scope.get("--color"), Some("red"));

        let mut child = Element::new("rect");
        child.set_attr("style", "--color: blue;"); // Override
        let child_scope = parent_scope.child_scope(&child);

        assert_eq!(child_scope.get("--color"), Some("blue"));
        assert_eq!(child_scope.get("--size"), Some("10px")); // Inherited
    }

    #[test]
    fn test_resolve_var() {
        let mut el = Element::new("g");
        el.set_attr("style", "--main-bg: #fff;");
        let scope = CssScope::new().child_scope(&el);

        assert_eq!(scope.resolve("var(--main-bg)"), "#fff");
        assert_eq!(scope.resolve("1px solid var(--main-bg)"), "1px solid #fff");
        assert_eq!(scope.resolve("var(--missing, black)"), "black");
    }
}
