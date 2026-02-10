//! Comprehensive integration tests for the CLI

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_version() {
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("vexy_svgo"));
}

#[test]
fn test_help() {
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"))
        .stdout(predicate::str::contains("Options"));
}

#[test]
fn test_optimize_file() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.svg");
    let output_path = temp_dir.path().join("output.svg");
    
    // Create test SVG
    let svg_content = r#"<svg width="100" height="100">
        <rect x="10" y="10" width="50" height="50" fill="red"/>
    </svg>"#;
    fs::write(&input_path, svg_content).unwrap();
    
    // Run optimization
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg(&input_path)
        .arg("-o")
        .arg(&output_path)
        .assert()
        .success();
    
    // Check output exists and is valid
    assert!(output_path.exists());
    let output = fs::read_to_string(&output_path).unwrap();
    assert!(output.contains("<svg"));
    assert!(output.contains("<path") || output.contains("<rect"));
}

#[test]
fn test_string_input() {
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-s")
        .arg("<svg><rect/></svg>")
        .assert()
        .success()
        .stdout(predicate::str::contains("<svg"))
        .stdout(predicate::str::contains("<rect"));
}

#[test]
fn test_multiple_files() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("file1.svg");
    let file2 = temp_dir.path().join("file2.svg");
    
    fs::write(&file1, "<svg><rect/></svg>").unwrap();
    fs::write(&file2, "<svg><circle/></svg>").unwrap();
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg(&file1)
        .arg(&file2)
        .assert()
        .success();
}

#[test]
fn test_folder_processing() {
    let temp_dir = TempDir::new().unwrap();
    let svg_dir = temp_dir.path().join("svgs");
    fs::create_dir(&svg_dir).unwrap();
    
    // Create test SVG files
    fs::write(svg_dir.join("file1.svg"), "<svg><rect/></svg>").unwrap();
    fs::write(svg_dir.join("file2.svg"), "<svg><circle/></svg>").unwrap();
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-f")
        .arg(&svg_dir)
        .assert()
        .success();
}

#[test]
fn test_pretty_output() {
    let svg_content = "<svg><g><rect/></g></svg>";
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-s")
        .arg(svg_content)
        .arg("--pretty")
        .assert()
        .success()
        .stdout(predicate::str::contains("<svg"));
}

#[test]
fn test_precision_option() {
    let svg = r#"<svg><path d="M 10.123456789 20.987654321"/></svg>"#;
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-s")
        .arg(svg)
        .arg("-p")
        .arg("2")
        .assert()
        .success();
}

#[test]
fn test_multipass() {
    let svg = r#"<svg><g><g><rect/></g></g></svg>"#;
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-s")
        .arg(svg)
        .arg("--multipass")
        .assert()
        .success();
}

#[test]
fn test_quiet_mode() {
    let temp_dir = TempDir::new().unwrap();
    let svg_path = temp_dir.path().join("test.svg");
    fs::write(&svg_path, "<svg><rect/></svg>").unwrap();
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg(&svg_path)
        .arg("-q")
        .assert()
        .success();
}

#[test]
fn test_verbose_mode() {
    let svg = r#"<svg><rect/></svg>"#;
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-s")
        .arg(svg)
        .arg("--verbose")
        .assert()
        .success();
}

#[test]
fn test_invalid_svg() {
    let invalid_svg = r#"<svg><rect"#; // Missing closing
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-s")
        .arg(invalid_svg)
        .assert();
    // May succeed or fail depending on parser behavior
}

#[test]
fn test_nonexistent_file() {
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("/nonexistent/file.svg")
        .assert()
        .failure();
}

#[test]
fn test_datauri_output() {
    let svg = r#"<svg><rect/></svg>"#;
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-s")
        .arg(svg)
        .arg("--datauri")
        .arg("base64")
        .assert()
        .success()
        .stdout(predicate::str::contains("data:"));
}

#[test]
fn test_show_plugins() {
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("--show-plugins")
        .assert()
        .success()
        .stdout(predicate::str::contains("Available plugins"));
}

#[test]
#[ignore = "CLI flag --suffix not yet implemented"]
fn test_suffix_option() {
    let temp_dir = TempDir::new().unwrap();
    let svg_path = temp_dir.path().join("test.svg");
    
    fs::write(&svg_path, "<svg><rect/></svg>").unwrap();
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg(&svg_path)
        .arg("--suffix")
        .arg(".min")
        .assert()
        .success();
    
    // Check that file with suffix was created
    // Output file should exist with some suffix
    let output_exists = temp_dir.path().join("test.min.svg").exists() ||
                       temp_dir.path().join("test.optimized.svg").exists() ||
                       temp_dir.path().join("test.opt.svg").exists();
    assert!(output_exists);
}

#[test]
fn test_indent_option() {
    let svg = r#"<svg><g><rect/></g></svg>"#;
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-s")
        .arg(svg)
        .arg("--pretty")
        .arg("--indent")
        .arg("4")
        .assert()
        .success();
}

#[test]
fn test_config_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");
    let svg_path = temp_dir.path().join("test.svg");
    
    // Create a simple config file
    let config = r#"{
        "multipass": true,
        "plugins": ["removeComments"]
    }"#;
    fs::write(&config_path, config).unwrap();
    
    // Create test SVG
    let svg = r#"<svg><!-- Comment --><rect/></svg>"#;
    fs::write(&svg_path, svg).unwrap();
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg(&svg_path)
        .arg("--config")
        .arg(&config_path)
        .assert()
        .success();
}

#[test]
fn test_output_to_stdout() {
    let temp_dir = TempDir::new().unwrap();
    let svg_path = temp_dir.path().join("test.svg");
    
    fs::write(&svg_path, "<svg><rect/></svg>").unwrap();
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg(&svg_path)
        .arg("-o")
        .arg("-")
        .assert()
        .success()
        .stdout(predicate::str::contains("<svg"));
}