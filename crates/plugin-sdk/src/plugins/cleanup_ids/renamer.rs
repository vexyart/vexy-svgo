// this_file: crates/plugin-sdk/src/plugins/cleanup_ids/renamer.rs

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

// Regex patterns for finding ID references
pub(crate) static REG_REFERENCES_URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\burl\(#([^)]+)\)"#).unwrap());
pub(crate) static REG_REFERENCES_URL_QUOTED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\burl\(["']#([^"']+)["']\)"#).unwrap());
pub(crate) static REG_REFERENCES_HREF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^#(.+?)$").unwrap());
pub(crate) static REG_REFERENCES_BEGIN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\w+)\.[a-zA-Z]").unwrap());

// Characters used for generating minified IDs
pub(crate) const GENERATE_ID_CHARS: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L',
    'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];

// Properties that can contain URL references
pub(crate) const REFERENCES_PROPS: &[&str] = &[
    "clip-path",
    "color-profile",
    "fill",
    "filter",
    "marker-end",
    "marker-mid",
    "marker-start",
    "mask",
    "stroke",
    "style",
];

/// ID generation state
#[derive(Debug, Clone)]
pub struct IdGenerator {
    current: Vec<usize>,
}

impl IdGenerator {
    pub fn new() -> Self {
        Self { current: vec![0] }
    }

    pub fn next(&mut self) -> String {
        let mut result = String::new();
        for &idx in &self.current {
            result.push(GENERATE_ID_CHARS[idx]);
        }

        // Increment
        let mut carry = true;
        for i in (0..self.current.len()).rev() {
            if carry {
                self.current[i] += 1;
                if self.current[i] >= GENERATE_ID_CHARS.len() {
                    self.current[i] = 0;
                } else {
                    carry = false;
                }
            }
        }
        if carry {
            self.current.push(0);
        }

        result
    }
}

/// Find all ID references in an attribute value
pub(crate) fn find_references(attr_name: &str, attr_value: &str) -> Vec<String> {
    let mut ids = Vec::new();

    // Check href attributes
    if attr_name == "href" || attr_name.ends_with(":href") {
        if let Some(captures) = REG_REFERENCES_HREF.captures(attr_value) {
            if let Some(id) = captures.get(1) {
                ids.push(id.as_str().to_string());
            }
        }
        return ids;
    }

    // Check begin attribute
    if attr_name == "begin" {
        for captures in REG_REFERENCES_BEGIN.captures_iter(attr_value) {
            if let Some(id) = captures.get(1) {
                ids.push(id.as_str().to_string());
            }
        }
        return ids;
    }

    // Check properties that can contain URL references
    if REFERENCES_PROPS.contains(&attr_name) || attr_name == "style" {
        // Check url(#id) patterns
        for captures in REG_REFERENCES_URL.captures_iter(attr_value) {
            if let Some(id) = captures.get(1) {
                ids.push(id.as_str().to_string());
            }
        }
        // Check url("#id") and url('#id') patterns
        for captures in REG_REFERENCES_URL_QUOTED.captures_iter(attr_value) {
            if let Some(id) = captures.get(1) {
                ids.push(id.as_str().to_string());
            }
        }
    }

    ids
}

/// Update references with new ID mappings
pub(crate) fn update_reference_value(value: &str, id_mappings: &HashMap<String, String>) -> String {
    let mut result = value.to_string();

    // Update plain #id references (for href attributes)
    result = REG_REFERENCES_HREF
        .replace_all(&result, |caps: &regex::Captures| {
            if let Some(id) = caps.get(1) {
                if let Some(new_id) = id_mappings.get(id.as_str()) {
                    return format!("#{}", new_id);
                }
            }
            caps[0].to_string()
        })
        .to_string();

    // Update url(#id) patterns
    result = REG_REFERENCES_URL
        .replace_all(&result, |caps: &regex::Captures| {
            if let Some(id) = caps.get(1) {
                if let Some(new_id) = id_mappings.get(id.as_str()) {
                    return format!("url(#{})", new_id);
                }
            }
            caps[0].to_string()
        })
        .to_string();

    // Update url("#id") and url('#id') patterns
    result = REG_REFERENCES_URL_QUOTED
        .replace_all(&result, |caps: &regex::Captures| {
            if let Some(id) = caps.get(1) {
                if let Some(new_id) = id_mappings.get(id.as_str()) {
                    // Preserve the quote style
                    let full_match = &caps[0];
                    let quote = if full_match.contains('"') { '"' } else { '\'' };
                    return format!("url({}#{}{})", quote, new_id, quote);
                }
            }
            caps[0].to_string()
        })
        .to_string();

    // Update begin="id.event" references.
    result = REG_REFERENCES_BEGIN
        .replace_all(&result, |caps: &regex::Captures| {
            if let Some(id) = caps.get(1) {
                if let Some(new_id) = id_mappings.get(id.as_str()) {
                    return caps[0].replacen(id.as_str(), new_id, 1);
                }
            }
            caps[0].to_string()
        })
        .to_string();

    result
}
