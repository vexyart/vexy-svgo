# Fuzzing

This document describes the fuzzing setup for security and robustness testing.

## Overview

Vexy SVGO uses [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) with libFuzzer to test critical parsers with arbitrary input, helping identify crashes, memory safety issues, and edge cases that unit tests might miss.

## Prerequisites

```bash
# Install cargo-fuzz
cargo install cargo-fuzz
```

## Fuzz Targets

### fuzz_svg_parser

Tests the main SVG parser with arbitrary input:
- UTF-8 and invalid UTF-8 strings
- Malformed XML/SVG
- Unexpected attribute values
- Large inputs

### fuzz_path_parser

Tests the SVG path data parser:
- Arbitrary path commands
- Invalid coordinate numbers
- Unexpected command sequences
- Malformed path syntax

### fuzz_css_parser

Tests CSS variable resolution:
- Arbitrary CSS strings
- Invalid var() syntax
- Circular references
- Unexpected input formats

## Running Fuzz Tests

### List all fuzz targets

```bash
cargo fuzz list
```

### Run a fuzz target (recommended)

```bash
# Run with default settings (stops on first crash)
cargo fuzz run fuzz_svg_parser

# Run with timeout (seconds)
cargo fuzz run fuzz_svg_parser -- -max_total_time=60

# Run for specific iterations
cargo fuzz run fuzz_svg_parser -- -runs=1000
```

### Build fuzz targets without running

```bash
cargo fuzz build
```

### Clean fuzz artifacts

```bash
cargo fuzz clean
```

## Corpus and Artifacts

Fuzzing generates working directories in `fuzz/artifacts/<target>/`:

- **corpus/**: Interesting inputs found during fuzzing
- **crash-***: Files that trigger crashes (if any)
- **leak-***: Files that cause memory leaks (if any)
- **timeout-***: Files that cause timeouts (if any)

When a crash is found, the triggering input is saved for debugging.

## Recommendations

### Local Development

Run fuzz targets for short durations during development:

```bash
# Quick 30-second check before commit
cargo fuzz run fuzz_svg_parser -- -max_total_time=30
```

### Periodic Testing

Run longer fuzz sessions periodically (e.g., overnight or weekly):

```bash
# Run for 2 hours
cargo fuzz run fuzz_svg_parser -- -max_total_time=7200
```

### CI Integration

For continuous integration, consider:
1. Running short fuzz sessions on each PR (30-60 seconds)
2. Running longer sessions on main branch builds
3. Using OSS-Fuzz for continuous automated fuzzing

## Interpreting Results

Crashes indicate potential:
- Memory safety issues (use-after-free, buffer overflow, etc.)
- Assertion failures
- Panic conditions
- Integer overflows/underflows

When a crash is found:
1. Examine the crash input file in `fuzz/artifacts/<target>/crash-*`
2. Reproduce locally: `cargo fuzz run <target> <crash-file>`
3. File a bug report with the crash input and reproduction steps
4. Fix the issue and verify the fix with the crash input

## Adding New Fuzz Targets

```bash
# Create new target
cargo fuzz add my_new_target

# Edit the generated file
vim fuzz/fuzz_targets/my_new_target.rs

# Build and run
cargo fuzz build my_new_target
cargo fuzz run my_new_target
```

## Example Fuzz Target

```rust
#![no_main]

use libfuzzer_sys::fuzz_target;
use vexy_svgo_core::some_function;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = some_function(input);
    }
});
```

## Resources

- [cargo-fuzz documentation](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- [libFuzzer documentation](https://llvm.org/docs/LibFuzzer.html)
- [Rust fuzzing book](https://rust-fuzz.github.io/book/)