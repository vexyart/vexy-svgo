// this_file: crates/plugin-sdk/src/plugin_test_macros.rs

extern crate vexy_svgo_core;


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
                    let mut config = Config::new();
                    config.plugins = vec![
                        vexy_svgo_core::PluginConfig::Name($plugin_name.to_string()),
                    ];
                    config.js2svg.pretty = true;
                    config.js2svg.indent = "    ".to_string();
                    config.js2svg.final_newline = false;

                    let registry = crate::registry::create_migrated_plugin_registry();
                    let options = vexy_svgo_core::OptimizeOptions::new(config).with_registry(registry);
                    let result = vexy_svgo_core::optimize(&fixture.input, options)?;
                    assert_eq!(result.data, fixture.expected, "Fixture: {} (Path: {:?})", fixture.name, fixtures_path.join(fixture.name.clone()));
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
                    let mut config = Config::new();
                    config.plugins = vec![
                        vexy_svgo_core::PluginConfig::Name($plugin_name.to_string()),
                    ];
                    if let Some(params) = fixture.params {
                        config.configure_plugin($plugin_name, params);
                    }
                    config.js2svg.pretty = true;
                    config.js2svg.indent = "    ".to_string();
                    config.js2svg.final_newline = false;

                    let registry = crate::registry::create_migrated_plugin_registry();
                    let options = vexy_svgo_core::OptimizeOptions::new(config).with_registry(registry);
                    let result = vexy_svgo_core::optimize(&fixture.input, options)?;
                    assert_eq!(result.data, fixture.expected, "Fixture: {} (Path: {:?})", fixture.name, fixtures_path.join(fixture.name.clone()));
                }
                Ok(())
            }
        }
    };
}
