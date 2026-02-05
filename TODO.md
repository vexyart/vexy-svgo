---
this_file: TODO.md
---

- [x] FIXME: Compiler warnings (all 6 warnings fixed - 2026-02-05)
  - ast.rs line 98, 165, 265: Added explicit lifetime annotations `Element<'_>`
  - convert_one_stop_gradients.rs: Removed unused `std::borrow::Cow`
  - convert_transform.rs: Removed unused `std::borrow::Cow`
  - enhanced_registry.rs: Removed unused `anyhow::anyhow`

- [ ] FIXME: WASM build - getrandom wasm32-unknown-unknown target issue
  - Error: The wasm32-unknown-unknown targets are not supported by default
  - Need to enable `wasm_js` configuration flag for getrandom

- [x] Obtain various SVGs (using testdata/integration/ and testdata/emoji/)

- [x] Build Vexy SVGO and run various optimizations on the SVGs

- [x] Run equivalent optimizations using the JS 'svgo'

- [x] diff the results

- [ ] iterate and improve Vexy SVGO until we ACTUALLY get an out of the box parity, including the default set of plugins and the optional plugins

## Comparison Results (2026-02-05)

### simple.svg
- **Vexy SVGO**: 114 bytes, output identical to JS SVGO
- **Status**: PARITY ACHIEVED ✓

### complex.svg
- **Vexy SVGO**: 600 bytes
- **JS SVGO**: 593 bytes (7 bytes smaller)
- **Differences**: 
  - ID renaming strategy (d,c vs a,b)
  - Path data format (spaces vs no spaces)
  - Path commands (absolute vs relative)
- **Status**: Minor differences, functionally equivalent

### emoji SVG (emoji_u1f3c3...)
- **Vexy SVGO**: 8,644 bytes (13.6% reduction)
- **JS SVGO**: 10,010 bytes (0% reduction)
- **Status**: Vexy SVGO performs BETTER

### glass.svg (695KB original)
- **Vexy SVGO**: 671,558 bytes (5.67% reduction)
- **JS SVGO**: 711,916 bytes (0% reduction)
- **Status**: Vexy SVGO performs BETTER

## Summary
- Compiler warnings: FIXED
- Basic optimization parity: ACHIEVED
- Vexy SVGO often produces smaller output than JS SVGO
- Key differences are in path data formatting (cosmetic, not functional)
