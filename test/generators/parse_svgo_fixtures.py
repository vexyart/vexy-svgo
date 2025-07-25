#!/usr/bin/env -S uv run
#  PROTECTED_0_ 
# dependencies = [
#    PROTECTED_0_ ,
# ]
#  PROTECTED_0_ 
# this_file: test/generators/parse_svgo_fixtures.py

"""
SVGO Fixture Parser and Converter

This script analyzes and converts SVGO test fixtures from ref/svgo/test/plugins/
into structured test data for VexySVGO compatibility testing.

According to PLAN.md Phase 2, this implements:
- Parse all .svg.txt files from ref/svgo/test/plugins/
- Extract input SVG, expected output SVG, and optional parameters
- Convert to structured format in testdata/plugins/{plugin_name}/
- Generate summary statistics for test coverage analysis
"""

import re
import json
from pathlib import Path
from typing import Dict, List, Tuple, Optional
from dataclasses import dataclass, asdict


@dataclass
class TestFixture:
    """Represents a single SVGO test fixture."""

    plugin_name: str
    test_number: str
    description: Optional[str]
    input_svg: str
    expected_svg: str
    params: Optional[Dict] = None


class SvgoFixtureParser:
    """Parses SVGO test fixtures and converts them to SVGN format."""

    def __init__(self, svgo_test_dir: Path, output_dir: Path):
        self.svgo_test_dir = svgo_test_dir
        self.output_dir = output_dir
        self.fixtures: List[TestFixture] = []

    def parse_fixture_file(self, file_path: Path) -> Optional[TestFixture]:
        """Parse a single SVGO .svg.txt fixture file."""
        try:
            content = file_path.read_text(encoding="utf-8")

            # Extract plugin name and test number from filename
            # Format: {plugin_name}.{number}.svg.txt
            match = re.match(r"([^.]+)\.(\d+)\.svg\.txt$", file_path.name)
            if not match:
                print(f"Warning: Unexpected filename format: {file_path.name}")
                return None

            plugin_name, test_number = match.groups()

            # Split content by @@@ separator
            parts = content.split("@@@")

            if len(parts) < 2:
                print(f"Warning: Invalid fixture format in {file_path.name}")
                return None

            input_svg = parts[0].strip()
            expected_svg = parts[1].strip()

            # Parse optional parameters from third part
            params = None
            if len(parts) >= 3:
                params_text = parts[2].strip()
                if params_text:
                    try:
                        params = json.loads(params_text)
                    except json.JSONDecodeError:
                        print(
                            f"Warning: Invalid JSON parameters in {file_path.name}: {params_text}"
                        )

            return TestFixture(
                plugin_name=plugin_name,
                test_number=test_number,
                description=None,  # SVGO format doesn't include descriptions
                input_svg=input_svg,
                expected_svg=expected_svg,
                params=params,
            )

        except Exception as e:
            print(f"Error parsing {file_path.name}: {e}")
            return None

    def scan_fixtures(self) -> None:
        """Scan and parse all SVGO fixture files."""
        plugins_dir = self.svgo_test_dir / "plugins"
        if not plugins_dir.exists():
            raise FileNotFoundError(
                f"SVGO plugins test directory not found: {plugins_dir}"
            )

        fixture_files = list(plugins_dir.glob("*.svg.txt"))
        print(f"Found {len(fixture_files)} SVGO fixture files")

        for file_path in sorted(fixture_files):
            fixture = self.parse_fixture_file(file_path)
            if fixture:
                self.fixtures.append(fixture)

        print(f"Successfully parsed {len(self.fixtures)} fixtures")

    def analyze_coverage(self) -> Dict[str, int]:
        """Analyze test coverage per plugin."""
        coverage = {}
        for fixture in self.fixtures:
            plugin = fixture.plugin_name
            coverage[plugin] = coverage.get(plugin, 0) + 1
        return coverage

    def convert_to_svgn_format(self) -> None:
        """Convert fixtures to SVGN testdata format."""
        # Group fixtures by plugin
        by_plugin = {}
        for fixture in self.fixtures:
            plugin = fixture.plugin_name
            if plugin not in by_plugin:
                by_plugin[plugin] = []
            by_plugin[plugin].append(fixture)

        # Create output directories and files
        for plugin_name, plugin_fixtures in by_plugin.items():
            plugin_dir = self.output_dir / "plugins" / plugin_name
            plugin_dir.mkdir(parents=True, exist_ok=True)

            for fixture in plugin_fixtures:
                # Create individual test case file
                test_file = plugin_dir / f"{fixture.test_number:0>2}.txt"

                # Write in SVGN format (compatible with existing fixture loader)
                content_parts = []
                if fixture.description:
                    content_parts.append(fixture.description)
                    content_parts.append("===")

                content_parts.append(fixture.input_svg)
                content_parts.append("@@@")
                content_parts.append(fixture.expected_svg)

                if fixture.params:
                    content_parts.append("@@@")
                    content_parts.append(json.dumps(fixture.params, indent=2))

                test_file.write_text("\n".join(content_parts) + "\n", encoding="utf-8")

        print(f"Converted fixtures for {len(by_plugin)} plugins to {self.output_dir}")

    def generate_summary_report(self) -> str:
        """Generate a summary report of the conversion process."""
        coverage = self.analyze_coverage()
        total_tests = sum(coverage.values())

        report = []
        report.append("# SVGO Fixture Migration Summary")
        report.append("")
        report.append(f"**Total Fixtures Processed:** {len(self.fixtures)}")
        report.append(f"**Total Plugins Covered:** {len(coverage)}")
        report.append(f"**Total Test Cases:** {total_tests}")
        report.append("")
        report.append("## Plugin Test Coverage")
        report.append("")
        report.append("| Plugin | Test Cases |")
        report.append("|--------|------------|")

        for plugin in sorted(coverage.keys()):
            test_count = coverage[plugin]
            report.append(f"| {plugin} | {test_count} |")

        report.append("")
        report.append("## Top 10 Most Tested Plugins")
        report.append("")

        sorted_plugins = sorted(coverage.items(), key=lambda x: x[1], reverse=True)[:10]
        for plugin, count in sorted_plugins:
            report.append(f"- **{plugin}**: {count} test cases")

        return "\n".join(report)


def main():
    """Main entry point for the SVGO fixture parser."""
    # Set up paths relative to project root
    project_root = Path(__file__).parent.parent.parent
    svgo_test_dir = project_root / "ref" / "svgo" / "test"
    output_dir = project_root / "testdata"

    print("SVGO Fixture Parser - Phase 2 Implementation")
    print("=" * 50)
    print(f"SVGO test directory: {svgo_test_dir}")
    print(f"Output directory: {output_dir}")
    print()

    # Initialize parser
    parser = SvgoFixtureParser(svgo_test_dir, output_dir)

    try:
        # Step 1: Scan and parse all fixtures
        print("Step 1: Scanning SVGO fixtures...")
        parser.scan_fixtures()
        print()

        # Step 2: Analyze coverage
        print("Step 2: Analyzing test coverage...")
        coverage = parser.analyze_coverage()
        print(f"Found tests for {len(coverage)} plugins")
        print()

        # Step 3: Convert to SVGN format
        print("Step 3: Converting to SVGN format...")
        parser.convert_to_svgn_format()
        print()

        # Step 4: Generate summary report
        print("Step 4: Generating summary report...")
        report = parser.generate_summary_report()

        report_file = output_dir / "svgo_fixture_migration_report.md"
        report_file.write_text(report, encoding="utf-8")
        print(f"Summary report written to: {report_file}")
        print()

        print("✅ SVGO fixture migration completed successfully!")
        print()
        print("Next steps:")
        print("1. Review the generated fixtures in testdata/plugins/")
        print("2. Run the Rust test generator to create plugin test files")
        print("3. Execute the test suite to validate SVGO compatibility")

    except Exception as e:
        print(f"❌ Error during fixture migration: {e}")
        return 1

    return 0


if __name__ == "__main__":
    exit(main())
