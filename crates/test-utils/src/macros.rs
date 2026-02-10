
// this_file: crates/test-utils/src/macros.rs

#[macro_export]
macro_rules! plugin_fixture_tests {
    ($plugin_struct:ident, $plugin_name:expr) => {
        #[cfg(test)]
        mod tests {
            use super::*;
            use vexy_svgo_core::Config;
            use vexy_svgo_test_utils::{load_fixtures, create_test_dir};
            use std::path::PathBuf;

            #[test]
            fn fixture_tests() -> Result<(), Box<dyn std::error::Error>> {
                let fixtures_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("..")
                    .join("..")
                    .join("testdata")
                    .join("plugins")
                    .join($plugin_name);

                if !fixtures_path.exists() {
                    println!("No fixtures found for plugin: {}", $plugin_name);
                    return Ok(());
                }

                let fixtures = load_fixtures(&fixtures_path)?;

                for fixture in fixtures {
                    let mut config = Config::with_default_preset();
                    config.set_plugin_enabled($plugin_name, true);
                    config.js2svg.final_newline = false;

                    let result = vexy_svgo_core::optimize_with_config(&fixture.input, config)?;
                    assert_eq!(result.data, fixture.expected, "Fixture: {}", fixture.name);
                }
                Ok(())
            }
        }
    };
}

#[macro_export]
macro_rules! plugin_fixture_tests_with_params {
    ($plugin_struct:ident, $plugin_name:expr) => {
        #[cfg(test)]
        mod tests {
            use super::*;
            use vexy_svgo_core::Config;
            use vexy_svgo_test_utils::{load_fixtures, create_test_dir};
            use std::path::PathBuf;

            #[test]
            fn fixture_tests_with_params() -> Result<(), Box<dyn std::error::Error>> {
                let fixtures_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("..")
                    .join("..")
                    .join("testdata")
                    .join("plugins")
                    .join($plugin_name);

                if !fixtures_path.exists() {
                    println!("No fixtures found for plugin: {}", $plugin_name);
                    return Ok(());
                }

                let fixtures = load_fixtures(&fixtures_path)?;

                for fixture in fixtures {
                    let mut config = Config::with_default_preset();
                    config.set_plugin_enabled($plugin_name, true);
                    config.js2svg.final_newline = false;
                    if let Some(params) = fixture.params {
                        config.configure_plugin($plugin_name, params);
                    }

                    let result = vexy_svgo_core::optimize_with_config(&fixture.input, config)?;
                    assert_eq!(result.data, fixture.expected, "Fixture: {}", fixture.name);
                }
                Ok(())
            }
        }
    };
}
