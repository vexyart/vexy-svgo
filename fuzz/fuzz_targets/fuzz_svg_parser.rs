// this_file: fuzz/fuzz_targets/fuzz_svg_parser.rs
//
// Fuzz target: Tests SVG parser for crashes/memory safety with arbitrary input

#![no_main]

use libfuzzer_sys::fuzz_target;
use vexy_svgo_core::parser::parse_svg;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = parse_svg(input);
    }
});
