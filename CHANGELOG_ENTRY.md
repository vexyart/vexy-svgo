# Feature Audit and Optimization Summary

**Date**: 2026-02-10

## Changes Made

### 1. Dependency Feature Optimization

**File Modified**: `crates/ffi/Cargo.toml`

**Change**: Disabled default features on `cbindgen` dependency

**Before**:
```toml
[dependencies]
cbindgen = "0.26.0"

[build-dependencies]
cbindgen = "0.26.0"
```

**After**:
```toml
[dependencies]
cbindgen = { version = "0.26.0", default-features = false }

[build-dependencies]
cbindgen = { version = "0.26.0", default-features = false }
```

**Rationale**: The "clap" feature enabled by default in cbindgen was unnecessary. cbindgen is only used for generating C headers during the build process, not as a CLI tool.

**Impact**:
- Dependency tree reduced from 801 to 791 lines (~1.2% reduction)
- Removed transitive dependencies: clap v3.2.25, indexmap v1.9.3, atty, textwrap, etc. (~6 packages)
- No functional changes - cbindgen works perfectly without the clap feature

### 2. Documentation Added

**New Files**:
1. `FEATURE_AUDIT.md` - Detailed dependency analysis
2. `FEATURES.md` - Comprehensive feature flag documentation

**Content**:
- Complete audit of all workspace dependencies and their features
- Analysis of which features are used vs unused
- Impact assessment for each optional feature
- Usage examples and recommendations
- Build command examples

## Verification Results

### Tests
- **Total Tests**: 877
- **Passed**: 877 ✅
- **Failed**: 0
- **Ignored**: 19 (intentionally skipped fixtures)

### Build
- **Release Build**: Success ✅
- **Build Time**: 1m 29s
- **Binary Size**: Unchanged (cbindgen is a build dependency only)

## Key Findings

### Well-Optimized Features
- **serde**: Uses `["derive"]` feature - optimal
- **clap**: Uses `["derive"]` feature - optimal
- **regex**: Default features (std, perf, unicode) are necessary
- **indexmap**: v2 with std default is appropriate

### Optional Features Already Properly Gated
- **json-schema**: Already optional, only enabled by CLI crate
- **parallel**: Available but not required for core functionality
- **size-optimization**: WASM-specific, properly optional

### Cleanup Candidate Found
- **cbindgen**: Had unnecessary "clap" feature - removed ✅

## Dependency Count

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Dependency tree lines | 801 | 791 | -10 (-1.2%) |
| Transitive deps from cbindgen | ~6 | 0 | -6 |
| Tests passing | 877 | 877 | ✅ Same |
| Build success | ✅ | ✅ | ✅ Same |

## Recommendations for Future

### Consideration: json-schema Feature Making Truly Optional
Currently, the CLI crate hardcodes `features = ["json-schema"]` on vexy-svgo-core. Consider:

1. Making this truly optional in the CLI
2. Adding a `no-jsonschema` feature for CLI
3. Fallback to basic validation when json-schema is disabled

**Impact**: Would allow building a smaller CLI without the reqwest/tokio ecosystem (~2MB savings).

**Trade-off**: Loss of strict SVGO configuration validation.

## No-Op Features Identified

The following feature flags exist but have no associated code:
- `lyon` (core)
- `experimental` (core)
- `memory-profiling` (core)
- `data-uri` (core)

These are placeholders for future development. No action needed.

## Files Changed

```
M crates/ffi/Cargo.toml     - Disabled cbindgen default features
A FEATURES.md                - Feature flag documentation
A FEATURE_AUDIT.md           - Dependency audit report
A CHANGELOG_ENTRY.md         - This file
```

---

**Result**: Successful feature audit and optimization with no functional changes, all tests passing, and dependency tree slightly reduced.