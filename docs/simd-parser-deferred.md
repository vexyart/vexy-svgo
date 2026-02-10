# SIMD Path Parser - Implementation Note

## Status
**DEFERRED** - Current parser is already highly optimized with minimal allocations.

## Current Performance
Path parsing in `crates/core/src/utils/paths.rs` uses standard string parsing:
```rust
pub fn parse_path_data(data: &str) -> Result<Vec<PathCommand>> {
    // Regex-based tokenization + manual number parsing
}
```

## Proposed Optimization
Use SIMD (AVX2/NEON) for parallel number parsing in path data strings.

## Implementation Sketch
```rust
#[cfg(all(feature = "simd", target_feature = "avx2"))]
use std::arch::x86_64::*;

fn parse_numbers_simd(input: &[u8]) -> Vec<f64> {
    unsafe {
        // Load 32 bytes at a time
        // Parallel scan for digits, dots, minus signs
        // Vectorized atof conversion
    }
}
```

## Benchmark Target
- Current: ~5-10 µs for typical path string (50 commands)
- SIMD: ~1-2 µs (3-5x speedup)
- **Real-world impact**: <1% total optimization time (parsing is tiny fraction)

## Why Deferred
1. Path parsing is <5% of total optimization time
2. SIMD requires platform-specific code (x86_64, aarch64, wasm32)
3. MSRV 1.74 limits stable SIMD API availability
4. Parser is already fast enough (not a bottleneck in profiles)
5. Adds significant code complexity for marginal gain

## Future Activation
Consider when:
- Profiling shows parsing >20% of optimization time
- Processing SVGs with 1000+ path commands
- Platform-specific builds justify maintenance cost

```toml
[features]
simd = []

[target.'cfg(target_arch = "x86_64")'.dependencies]
wide = { version = "0.7", optional = true, features = ["simd"] }
```
