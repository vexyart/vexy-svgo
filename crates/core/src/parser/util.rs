// this_file: crates/core/src/parser/util.rs

use std::collections::HashSet;
use std::sync::OnceLock;

/// Elements where whitespace should be preserved
pub(crate) static TEXT_ELEMENTS: OnceLock<HashSet<&'static str>> = OnceLock::new();

pub(crate) fn text_elements() -> &'static HashSet<&'static str> {
    TEXT_ELEMENTS.get_or_init(|| {
        HashSet::from([
            // Text content elements
            "text",
            "tspan",
            "textPath",
            "altGlyph",
            "tref",
            "glyph",
            "glyphRef",
            "altGlyphDef",
            "altGlyphItem",
            // Other elements that need whitespace preservation
            "pre",
            "title",
            "script",
            "style",
        ])
    })
}
