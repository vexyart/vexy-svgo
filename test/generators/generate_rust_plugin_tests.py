#!/usr/bin/env -S uv run
#  PROTECTED_0_ 
# dependencies = [
#    PROTECTED_0_ ,
# ]
#  PROTECTED_0_ 
# this_file: test/generators/generate_rust_plugin_tests.py

"""
Generate Rust test files for VexySVGO plugins using converted SVGO fixtures.

This script reads all plugin directories from testdata/plugins/ and generates
corresponding Rust test files in vexy_svgo/tests/plugins/ following the established
pattern from existing test files.

Usage:
    python test/generators/generate_rust_plugin_tests.py

The script will:
1. Scan testdata/plugins/ for all plugin directories
2. Check which plugins already have test files in vexy_svgo/tests/plugins/
3. Generate missing test files using the fixture data
4. Follow the established pattern from cleanup_ids.rs
"""

from pathlib import Path
import re


def convert_plugin_name_to_snake_case(plugin_name: str) -> str:
    """
    Convert plugin name from camelCase to snake_case for Rust file naming.

    Examples:
        cleanupIds -> cleanup_ids
        addAttributesToSVGElement -> add_attributes_to_svg_element
        inlineStyles -> inline_styles
    """
    # Handle SVG as a special case (SVGElement -> svg_element)
    result = re.sub(r"SVG", "Svg", plugin_name)

    # Insert underscores before capital letters
    result = re.sub(r"([a-z])([A-Z])", r"\1_\2", result)

    # Convert to lowercase
    return result.lower()


def get_fixture_files(plugin_dir: Path) -> list[str]:
    """Get all .txt fixture files in a plugin directory, sorted by name."""
    if not plugin_dir.exists():
        return []

    fixture_files = [f.stem for f in plugin_dir.glob("*.txt")]
    # Sort numerically if possible, otherwise alphabetically
    try:
        fixture_files.sort(key=lambda x: int(x))
    except ValueError:
        fixture_files.sort()

    return fixture_files


def parse_fixture_file(fixture_path: Path) -> dict:
    """Parse a fixture file to extract description, input, expected, and params."""
    if not fixture_path.exists():
        return {}

    content = fixture_path.read_text(encoding="utf-8")
    normalized = content.strip().replace("\r\n", "\n")

    # Split by === to separate description from test case
    parts = normalized.split("\n===\n")
    if len(parts) == 2:
        description = parts[0].strip()
        test_content = parts[1]
    else:
        description = None
        test_content = normalized

    # Split test content by @@@
    test_parts = test_content.split("\n@@@\n")

    if len(test_parts) < 2:
        return {}

    input_svg = test_parts[0].strip()
    expected_svg = test_parts[1].strip()

    params = None
    if len(test_parts) > 2 and test_parts[2].strip():
        params = test_parts[2].strip()

    return {
        "description": description,
        "input": input_svg,
        "expected": expected_svg,
        "params": params,
    }


def generate_test_function(
    plugin_name: str, fixture_name: str, fixture_data: dict
) -> str:
    """Generate a single test function for a fixture."""
    # Create test function name
    test_name = f"test_{convert_plugin_name_to_snake_case(plugin_name)}_{fixture_name}"

    # Create description comment
    description = fixture_data.get("description") or ""
    description = description.replace("\n", "\n    // ") if description else ""
    description_comment = f"    // {description}" if description else ""

    # Format input and expected SVG with proper escaping
    input_svg = (
        (fixture_data.get("input") or "").replace("\\", "\\\\").replace(' PROTECTED_4_ ')
    )
    expected_svg = (
        (fixture_data.get("expected") or "").replace("\\", "\\\\").replace(' PROTECTED_4_ ')
    )

    # Handle parameters
    params_line = ""
    if fixture_data.get("params"):
        params_line = f'    let params = serde_json::from_str(r# PROTECTED_0_ params PROTECTED_1_ #).ok();'
    else:
        params_line = "    let params = None;"

    return f'''#[test]
fn {test_name}() {{
{description_comment}
    let input = r# PROTECTED_0_ #;

    let expected = r# PROTECTED_0_ #;

{params_line}

    test_plugin(input, expected, params);
}}'''


def generate_rust_test_file(
    plugin_name: str, fixture_files: list[str], testdata_dir: Path
) -> str:
    """Generate a complete Rust test file for a plugin."""
    snake_case_name = convert_plugin_name_to_snake_case(plugin_name)

    # File header with magic comment
    header = f'''// this_file: test/plugins/{snake_case_name}.rs

//! Tests for the {plugin_name} plugin
//! Auto-generated from SVGO test fixtures

use vexy_svgo::{{optimize, OptimizeOptions, Config, PluginConfig}};
use vexy_svgo::config::Js2SvgOptions;
use serde_json::json;

fn test_plugin(input: &str, expected: &str, params: Option<serde_json::Value>) {{
    let config = Config {{
        plugins: vec![PluginConfig {{
            name: "{plugin_name}".to_string(),
            params,
        }}],
        multipass: false,
        js2svg: Js2SvgOptions {{
            pretty: true,
            indent: 4,
            ..Default::default()
        }},
        ..Default::default()
    }};

    let options = OptimizeOptions::new(config);
    let result = optimize(input, options).expect("Optimization should succeed");
    let output = result.data.trim();
    let expected = expected.trim();
    
    assert_eq!(output, expected, 
        "\\nPlugin: {plugin_name}\\nInput:\\n{{}}\\nExpected:\\n{{}}\\nActual:\\n{{}}\\n", 
        input, expected, output);
}}

'''

    # Generate test functions
    test_functions = []
    plugin_dir = testdata_dir / plugin_name

    for fixture_name in fixture_files:
        fixture_path = plugin_dir / f"{fixture_name}.txt"
        fixture_data = parse_fixture_file(fixture_path)

        if fixture_data and fixture_data.get("input") and fixture_data.get("expected"):
            test_function = generate_test_function(
                plugin_name, fixture_name, fixture_data
            )
            test_functions.append(test_function)

    return header + "\n\n".join(test_functions) + "\n"


def main():
    """Main function to generate all missing plugin test files."""
    # Project paths
    project_root = Path(__file__).parent.parent.parent
    testdata_dir = project_root / "testdata" / "plugins"
    tests_dir = project_root / "svgn" / "tests" / "plugins"

    print(f"Scanning testdata directory: {testdata_dir}")
    print(f"Target tests directory: {tests_dir}")

    if not testdata_dir.exists():
        print(f"ERROR: testdata directory not found: {testdata_dir}")
        return

    # Ensure tests directory exists
    tests_dir.mkdir(parents=True, exist_ok=True)

    # Get all plugin directories
    plugin_dirs = [d for d in testdata_dir.iterdir() if d.is_dir()]
    plugin_names = [d.name for d in plugin_dirs]
    plugin_names.sort()

    print(f"Found {len(plugin_names)} plugins in testdata")

    # Check existing test files
    existing_tests = set()
    for test_file in tests_dir.glob("*.rs"):
        # Convert filename back to plugin name
        snake_name = test_file.stem
        # This is a simplified reverse conversion - may need refinement
        plugin_name = "".join(word.capitalize() for word in snake_name.split("_"))
        existing_tests.add(plugin_name)

    print(f"Found {len(existing_tests)} existing test files")

    # Generate missing test files
    generated_count = 0
    skipped_count = 0

    for plugin_name in plugin_names:
        snake_case_name = convert_plugin_name_to_snake_case(plugin_name)
        test_file_path = tests_dir / f"{snake_case_name}.rs"

        # Skip if test file already exists
        if test_file_path.exists():
            print(f"SKIP: {plugin_name} (test file already exists)")
            skipped_count += 1
            continue

        # Get fixture files for this plugin
        fixture_files = get_fixture_files(testdata_dir / plugin_name)

        if not fixture_files:
            print(f"SKIP: {plugin_name} (no fixture files found)")
            skipped_count += 1
            continue

        # Generate test file content
        test_content = generate_rust_test_file(plugin_name, fixture_files, testdata_dir)

        # Write test file
        test_file_path.write_text(test_content, encoding="utf-8")

        print(
            f"GENERATED: {plugin_name} -> {snake_case_name}.rs ({len(fixture_files)} tests)"
        )
        generated_count += 1

    print(f"\nSummary:")
    print(f"- Generated: {generated_count} new test files")
    print(f"- Skipped: {skipped_count} existing/empty plugins")
    print(f"- Total plugins: {len(plugin_names)}")

    if generated_count > 0:
        print(f"\nNext steps:")
        print(f"1. Run 'cargo test' to verify all tests compile and pass")
        print(f"2. Review generated test files for any needed adjustments")
        print(
            f"3. Update PLAN.md and TODO.md to mark 'Generate Rust test files' as complete"
        )


if __name__ == "__main__":
    main()
