// this_file: crates/test-utils/src/lib.rs

//! # vexy_svgo-test-utils
//!
//! Shared testing utilities for vexy_svgo.

use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Test fixture structure
#[derive(Debug)]
pub struct TestFixture {
    pub name: String,
    pub input: String,
    pub expected: String,
    pub params: Option<serde_json::Value>,
}

/// Load test fixtures from a directory
pub fn load_fixtures(dir: &Path) -> Result<Vec<TestFixture>, Box<dyn std::error::Error>> {
    let mut fixtures = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("txt") {
            let content = fs::read_to_string(&path)?;
            if let Some(fixture) = parse_fixture(&content) {
                fixtures.push(fixture);
            }
        }
    }

    Ok(fixtures)
}

/// Parse a fixture file in SVGO format
fn parse_fixture(content: &str) -> Option<TestFixture> {
    let parts: Vec<&str> = content.split("@@@").collect();
    if parts.len() < 2 {
        return None;
    }

    let input = parts[0].trim().to_string();
    let expected = parts[1].trim().to_string();
    let params = parts
        .get(2)
        .and_then(|p| serde_json::from_str(p.trim()).ok());

    Some(TestFixture {
        name: String::new(), // Will be set from filename
        input,
        expected,
        params,
    })
}

/// Create a temporary directory with test files
pub fn create_test_dir() -> Result<TempDir, Box<dyn std::error::Error>> {
    Ok(TempDir::new()?)
}

#[macro_use]
pub mod test_macros;

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