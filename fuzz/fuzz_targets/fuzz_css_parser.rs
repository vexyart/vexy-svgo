// this_file: fuzz/fuzz_targets/fuzz_css_parser.rs
// Fuzz target: Tests CSS variable parser for crashes/memory safety
#![no_main]

use libfuzzer_sys::fuzz_target;
use vexy_svgo_core::{CssScope, CssVariableResolver};

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let scope = CssScope::new();
        let _ = scope.resolve(input);
        let _ = CssVariableResolver::resolve_value(input, &std::collections::HashMap::new());
    }
});
