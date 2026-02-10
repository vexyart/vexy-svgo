// this_file: crates/cli/tests/additional_integration_tests.rs

//! Additional integration tests for newer CLI features

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_parallel_processing() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create multiple SVG files
    for i in 0..10 {
        let path = temp_dir.path().join(format!("file{}.svg", i));
        fs::write(&path, format!(r#"<svg id="file{}"><rect width="100" height="100"/></svg>"#, i)).unwrap();
    }
    
    // Test with explicit parallel processing
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-f")
        .arg(temp_dir.path())
        .arg("--parallel")
        .arg("4")
        .assert()
        .success();
}

#[test]
#[ignore = "CLI flag --memory-limit not yet implemented"]
fn test_memory_limit() {
    let svg = r#"<svg><rect width="100" height="100"/></svg>"#;
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-s")
        .arg(svg)
        .arg("--memory-limit")
        .arg("100MB")
        .assert()
        .success();
}

#[test]
#[ignore = "CLI flag --streaming not yet implemented"]
fn test_streaming_mode() {
    let temp_dir = TempDir::new().unwrap();
    let large_svg_path = temp_dir.path().join("large.svg");
    
    // Create a "large" SVG (simulate with many elements)
    let mut svg_content = String::from("<svg>");
    for i in 0..1000 {
        svg_content.push_str(&format!(r#"<rect id="r{}" x="{}" y="{}" width="10" height="10"/>"#, i, i % 100, i / 100));
    }
    svg_content.push_str("</svg>");
    fs::write(&large_svg_path, svg_content).unwrap();
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg(&large_svg_path)
        .arg("--streaming")
        .assert()
        .success();
}

#[test]
fn test_plugin_enable_disable() {
    let svg = r#"<svg><!-- Comment to remove --><rect/></svg>"#;
    
    // Test with removeComments disabled
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-s")
        .arg(svg)
        .arg("--disable")
        .arg("removeComments")
        .assert()
        .success()
        .stdout(predicate::str::contains("<!-- Comment"));
    
    // Test with removeComments enabled
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-s")
        .arg(svg)
        .arg("--enable")
        .arg("removeComments")
        .assert()
        .success()
        .stdout(predicate::str::contains("<!-- Comment").not());
}

#[test]
#[ignore = "CLI flag --plugin-params not yet implemented"]
fn test_plugin_params() {
    let svg = r#"<svg><path d="M 10.123456789 20.987654321 L 30.111111111 40.222222222"/></svg>"#;
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-s")
        .arg(svg)
        .arg("--enable")
        .arg("convertPathData")
        .arg("--plugin-params")
        .arg("convertPathData.floatPrecision=2")
        .assert()
        .success();
}

#[test]
#[ignore = "CLI flag --benchmark not yet implemented"]
fn test_benchmark_mode() {
    let svg = r#"<svg><rect/></svg>"#;
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-s")
        .arg(svg)
        .arg("--benchmark")
        .assert()
        .success()
        .stdout(predicate::str::contains("Processing time"));
}

#[test]
#[ignore = "CLI flag --timing not yet implemented"]
fn test_timing_output() {
    let svg = r#"<svg><rect/></svg>"#;
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-s")
        .arg(svg)
        .arg("--timing")
        .assert()
        .success();
}

#[test]
#[ignore = "CLI flag --output-format not yet implemented"]
fn test_json_output() {
    let svg = r#"<svg><rect width="100" height="100"/></svg>"#;
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-s")
        .arg(svg)
        .arg("--output-format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("}"));
}

#[test]
#[ignore = "CLI flag --batch-size not yet implemented"]
fn test_batch_size_option() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create multiple SVG files
    for i in 0..20 {
        let path = temp_dir.path().join(format!("file{}.svg", i));
        fs::write(&path, format!(r#"<svg id="{}"><rect/></svg>"#, i)).unwrap();
    }
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-f")
        .arg(temp_dir.path())
        .arg("--batch-size")
        .arg("5")
        .assert()
        .success();
}

#[test]
#[ignore = "CLI flag --watch not yet implemented"]
fn test_watch_mode() {
    // Note: Watch mode would need special handling for testing
    // This test just verifies the option is recognized
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("watch"));
}

#[test]
#[ignore = "CLI flag --stats not yet implemented"]
fn test_stats_output() {
    let temp_dir = TempDir::new().unwrap();
    let svg_path = temp_dir.path().join("test.svg");
    
    fs::write(&svg_path, r#"<svg><!-- Remove me --><rect width="100" height="100"/></svg>"#).unwrap();
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg(&svg_path)
        .arg("--stats")
        .assert()
        .success()
        .stdout(predicate::str::contains("Original size"))
        .stdout(predicate::str::contains("Optimized size"));
}

#[test]
fn test_preserve_important_comments() {
    let svg = r#"<svg><!--! Important comment --><rect/></svg>"#;
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-s")
        .arg(svg)
        .assert()
        .success()
        .stdout(predicate::str::contains("<!--!"));
}

#[test]
#[ignore = "CLI flag --glob not yet implemented"]
fn test_glob_pattern() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create SVG files with different extensions
    fs::write(temp_dir.path().join("icon1.svg"), "<svg><rect/></svg>").unwrap();
    fs::write(temp_dir.path().join("icon2.svg"), "<svg><circle/></svg>").unwrap();
    fs::write(temp_dir.path().join("ignore.txt"), "not an svg").unwrap();
    
    let glob_pattern = temp_dir.path().join("*.svg");
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("--glob")
        .arg(glob_pattern.to_str().unwrap())
        .assert()
        .success();
}

#[test]
#[ignore = "CLI flag --continue-on-error not yet implemented"]
fn test_error_recovery() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create one valid and one invalid SVG
    fs::write(temp_dir.path().join("valid.svg"), "<svg><rect/></svg>").unwrap();
    fs::write(temp_dir.path().join("invalid.svg"), "<svg><rect").unwrap(); // Invalid XML
    
    // Should process valid file even if one fails
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-f")
        .arg(temp_dir.path())
        .arg("--continue-on-error")
        .assert();
    // Success depends on error handling mode
}

#[test]
#[ignore = "CLI flag --progress not yet implemented"]
fn test_progress_indicator() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create multiple files to trigger progress indicator
    for i in 0..50 {
        fs::write(
            temp_dir.path().join(format!("file{}.svg", i)),
            format!(r#"<svg id="f{}"><rect/></svg>"#, i)
        ).unwrap();
    }
    
    let mut cmd = Command::cargo_bin("vexy-svgo").unwrap();
    cmd.arg("-f")
        .arg(temp_dir.path())
        .arg("--progress")
        .assert()
        .success();
}