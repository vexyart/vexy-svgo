// this_file: fuzz/fuzz_targets/fuzz_path_parser.rs
// Fuzz target: Tests path data parser for crashes/memory safety
#![no_main]

use libfuzzer_sys::fuzz_target;
use vexy_svgo_core::utils::paths::PathUtils;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = PathUtils::parse_path_data(input);
    }
});
