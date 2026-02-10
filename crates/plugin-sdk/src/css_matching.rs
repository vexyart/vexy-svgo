// this_file: crates/plugin-sdk/src/css_matching.rs

use cssparser::ToCss;
use precomputed_hash::PrecomputedHash;
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::matching::{ElementSelectorFlags, MatchingContext};
use selectors::{Element, OpaqueElement, SelectorImpl};
use std::borrow::Borrow;
use std::fmt;
use vexy_svgo_core::ast::{self, Node};

/// Wrapper newtype for String that implements the traits required by selectors 0.30:
/// ToCss, PrecomputedHash, From<&str>, Clone, Eq, Borrow<str>, Default.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CssString(pub String);

impl From<&str> for CssString {
    fn from(s: &str) -> Self {
        CssString(s.to_owned())
    }
}

impl ToCss for CssString {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        dest.write_str(&self.0)
    }
}

impl PrecomputedHash for CssString {
    fn precomputed_hash(&self) -> u32 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        hasher.finish() as u32
    }
}

impl Borrow<str> for CssString {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Default for CssString {
    fn default() -> Self {
        CssString(String::new())
    }
}

impl AsRef<str> for CssString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Selector implementation types for Vexy SVGO
#[derive(Debug, Clone)]
pub struct VexySelectorImpl;

impl SelectorImpl for VexySelectorImpl {
    type AttrValue = CssString;
    type Identifier = CssString;
    type LocalName = CssString;
    type NamespaceUrl = CssString;
    type NamespacePrefix = CssString;
    type BorrowedNamespaceUrl = str;
    type BorrowedLocalName = str;
    type NonTSPseudoClass = NonTSPseudoClass;
    type PseudoElement = PseudoElement;
    type ExtraMatchingData<'a> = ();
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NonTSPseudoClass {}

impl selectors::parser::NonTSPseudoClass for NonTSPseudoClass {
    type Impl = VexySelectorImpl;
    fn is_active_or_hover(&self) -> bool {
        false
    }
    fn is_user_action_state(&self) -> bool {
        false
    }
}

impl ToCss for NonTSPseudoClass {
    fn to_css<W>(&self, _dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PseudoElement {}

impl selectors::parser::PseudoElement for PseudoElement {
    type Impl = VexySelectorImpl;
}

impl ToCss for PseudoElement {
    fn to_css<W>(&self, _dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        Ok(())
    }
}

/// Wrapper around an element that includes its ancestry path
#[derive(Debug, Clone)]
pub struct ElementWrapper<'a> {
    /// Path from root to this element (inclusive)
    pub path: Vec<&'a ast::Element<'a>>,
}

impl<'a> ElementWrapper<'a> {
    pub fn new(root: &'a ast::Element<'a>) -> Self {
        Self {
            path: vec![root],
        }
    }

    pub fn current(&self) -> &'a ast::Element<'a> {
        self.path.last().expect("Path cannot be empty")
    }
}

impl<'a> Element for ElementWrapper<'a> {
    type Impl = VexySelectorImpl;

    fn opaque(&self) -> OpaqueElement {
        // Use the address of the underlying Element as unique ID
        OpaqueElement::new(self.current())
    }

    fn parent_element(&self) -> Option<Self> {
        if self.path.len() > 1 {
            let mut new_path = self.path.clone();
            new_path.pop();
            Some(Self { path: new_path })
        } else {
            None
        }
    }

    fn parent_node_is_shadow_root(&self) -> bool {
        false
    }

    fn containing_shadow_host(&self) -> Option<Self> {
        None
    }

    fn is_pseudo_element(&self) -> bool {
        false
    }

    fn first_element_child(&self) -> Option<Self> {
        self.current().children.iter().find_map(|node| {
            if let Node::Element(el) = node {
                let mut new_path = self.path.clone();
                new_path.push(el);
                Some(Self { path: new_path })
            } else {
                None
            }
        })
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        // This operation is O(N) where N is the number of siblings
        // because we must traverse from the parent to find the current node's index.
        if self.path.len() <= 1 {
            return None;
        }

        let parent = self.path[self.path.len() - 2];
        let current_ptr = self.current() as *const _;

        let mut prev_el: Option<&'a ast::Element<'a>> = None;

        for child in &parent.children {
            if let Node::Element(el) = child {
                if el as *const _ == current_ptr {
                    if let Some(prev) = prev_el {
                        let mut new_path = self.path.clone();
                        new_path.pop();
                        new_path.push(prev);
                        return Some(Self { path: new_path });
                    }
                    return None;
                }
                prev_el = Some(el);
            }
        }
        None
    }

    fn next_sibling_element(&self) -> Option<Self> {
        if self.path.len() <= 1 {
            return None;
        }

        let parent = self.path[self.path.len() - 2];
        let current_ptr = self.current() as *const _;
        let mut found_self = false;

        for child in &parent.children {
            if let Node::Element(el) = child {
                if found_self {
                    let mut new_path = self.path.clone();
                    new_path.pop();
                    new_path.push(el);
                    return Some(Self { path: new_path });
                }
                if el as *const _ == current_ptr {
                    found_self = true;
                }
            }
        }
        None
    }

    fn is_html_element_in_html_document(&self) -> bool {
        false
    }

    fn has_local_name(&self, local_name: &<Self::Impl as SelectorImpl>::BorrowedLocalName) -> bool {
        self.current().name == local_name
    }

    fn has_namespace(&self, _ns: &<Self::Impl as SelectorImpl>::BorrowedNamespaceUrl) -> bool {
        // Namespace support is minimal for now as SVGO selectors typically ignore namespaces.
        true
    }

    fn is_same_type(&self, other: &Self) -> bool {
        self.current().name == other.current().name
    }

    fn attr_matches(
        &self,
        _ns: &NamespaceConstraint<&<Self::Impl as SelectorImpl>::NamespaceUrl>,
        local_name: &<Self::Impl as SelectorImpl>::LocalName,
        operation: &AttrSelectorOperation<&<Self::Impl as SelectorImpl>::AttrValue>,
    ) -> bool {
        // Attribute lookup ignoring namespace constraint for simplicity
        if let Some(value) = self.current().attr(&local_name.0) {
             operation.eval_str(value)
        } else {
            false
        }
    }

    fn match_non_ts_pseudo_class(
        &self,
        _pc: &<Self::Impl as SelectorImpl>::NonTSPseudoClass,
        _context: &mut MatchingContext<Self::Impl>,
    ) -> bool {
        false
    }

    fn match_pseudo_element(
        &self,
        _pe: &<Self::Impl as SelectorImpl>::PseudoElement,
        _context: &mut MatchingContext<Self::Impl>,
    ) -> bool {
        false
    }

    fn apply_selector_flags(&self, _flags: ElementSelectorFlags) {}

    fn is_link(&self) -> bool {
        self.current().name == "a"
    }

    fn is_html_slot_element(&self) -> bool {
        false
    }

    fn has_id(
        &self,
        id: &<Self::Impl as SelectorImpl>::Identifier,
        _case_sensitivity: CaseSensitivity,
    ) -> bool {
        self.current().attr("id") == Some(id.0.as_str())
    }

    fn has_class(
        &self,
        name: &<Self::Impl as SelectorImpl>::Identifier,
        case_sensitivity: CaseSensitivity,
    ) -> bool {
        if let Some(class_attr) = self.current().attr("class") {
            class_attr.split_whitespace().any(|c| case_sensitivity.eq(c.as_bytes(), name.0.as_bytes()))
        } else {
            false
        }
    }

    fn imported_part(&self, _name: &<Self::Impl as SelectorImpl>::Identifier) -> Option<<Self::Impl as SelectorImpl>::Identifier> {
        None
    }

    fn is_part(&self, _name: &<Self::Impl as SelectorImpl>::Identifier) -> bool {
        false
    }

    fn has_custom_state(&self, _name: &<Self::Impl as SelectorImpl>::Identifier) -> bool {
        false
    }

    fn add_element_unique_hashes(
        &self,
        _filter: &mut selectors::bloom::CountingBloomFilter<selectors::bloom::BloomStorageU8>,
    ) -> bool {
        false
    }

    fn is_empty(&self) -> bool {
        self.current().is_empty()
    }

    fn is_root(&self) -> bool {
        self.path.len() == 1
    }
}
