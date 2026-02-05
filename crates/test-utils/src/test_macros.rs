// this_file: test/test_macro.rs

//! Macro to generate plugin test cases from fixtures.
//! This file is intended to be included directly in integration test files.
extern crate vexy_svgo_core;
// This file is intended to be included directly in integration test files.

/// Macro to generate plugin test cases from fixtures.
/// Expects `testdata/plugins/{plugin_name}/` to contain .txt files with SVGO-style fixtures.
///
/// Usage:
/// ```ignore
/// generate_plugin_tests!("cleanupAttrs");
/// generate_plugin_tests!("convertColors");
/// ```
#[macro_export]
macro_rules! generate_plugin_tests {
    ($plugin_name:expr) => {
        paste::item! {
            #[test]
            fn [<test_ $plugin_name _fixtures>]() {
                // The fixtures function expects just the plugin name, not the full path
                let fixtures = vexy_svgo_core::fixtures::load_plugin_fixtures($plugin_name)
                    .expect(&format!("Failed to load fixtures for {}", $plugin_name));

                for fixture in fixtures {
                    println!("Running test for {}: {}", $plugin_name, fixture.name);
                    vexy_svgo_core::test_utils::run_plugin_fixture_test($plugin_name, fixture);
                }
            }
        }
    };
    ($($plugin_name:expr),*) => {
        $(
            generate_plugin_tests!($plugin_name);
        )*
    };
}
