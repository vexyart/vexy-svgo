# Vexy SVGO Development Plan

## Completed

### Issue #201: SVGO Default Plugin Parity ✅ (2025-07-25)
All 33 SVGO default plugins implemented and enabled. 100% parity achieved.

### usvg Plugin Integration ✅ (2026-02-09)
Integrated usvg (resvg project) as an opt-in plugin for SVG normalization.
Plugin registered, tested (14/14 pass), graceful error handling implemented.

### WASM Build Fix (issue 125) ✅ (previously applied)
Build scripts already include `--cfg getrandom_backend="wasm_js"` in RUSTFLAGS.

## Strategic Roadmap: Parallel Execution Waves

### Wave 1: Foundation & Performance (Core Blockers)
*Focus: Eliminating architectural bottlenecks and establishing infrastructure.*
- **Zero-Copy Parser & AST Arena**: Essential for performance scaling. Must precede advanced plugin logic to avoid double refactoring.
- **CSS Variable Resolution**: Fundamental for correct style minification.
- **Infrastructure Audits**: WASM size and MSRV are baseline requirements for distribution.

### Wave 2: Intelligence & Advanced Plugins
*Focus: Advanced SVG optimization logic and stabilization.*
- **MinifyStyles Tree-Shaking**: Depends on Wave 1 CSS resolver.
- **Spatial Indexing & SIMD**: Optimization of already established core paths.
- **CI Benchmarking**: Once core is stable, we lock in performance gains.

### Wave 3: Ecosystem, DX & Bindings
*Focus: Distribution, tooling, and multi-language support.*
- **WASM Wrappers & CLI DX**: Building on the high-performance core.
- **FFI Bindings**: Stabilizing interfaces for Python/Go.

## Future Considerations

- Implement remaining 11 CLI features (watch, benchmark, streaming, etc.)
- Resolve 31 pre-existing fixture test differences (path optimization, ID renaming)
- Consider usvg version bump (0.45.1 → 0.47.0) for latest features
- Extend fixture test macro to support opt-in plugins
- Maintain parity with future SVGO updates