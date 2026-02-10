# Dependency Feature Audit

## Summary

This document describes the dependency features currently used across the Vexy SVGO workspace and identifies optimization opportunities.

## Analysis Results

### Statistics
- Total dependency tree lines: 801
- Workspace crates: 7 (core, plugin-sdk, cli, wasm, ffi, benchmarks, test-utils)
- Total identified feature flags: 9 (across workspace and individual crates)

### Key Findings

#### 1. **jsonschema Feature (vexy-svgo-core)**
- **Status**: Optional feature `json-schema`
- **Current Usage**: Used by cli crate for config validation in `src/parser/config.rs`
- **Transitive Dependencies**:
  - reqwest v0.11.27 (HTTP client)
  - tokio v1.46.1 (async runtime)
  - h2 v0.3.27 (HTTP/2)
  - hyper v0.14.32 (HTTP implementation)
  - serde_urlencoded, base64 v0.21.7, bytes, etc.
- **Feature Dependencies**: `default` features of jsonschema (draft201909, draft202012, resolve-file, resolve-http)
- **Recommendation**: Keep as optional feature (already gated with `#[cfg(feature = "json-schema")]`)
- **Note**: The plugin command files exist in `crates/cli/src/commands/plugin/` but are not wired up in `main.rs`

#### 2. **cbindgen (vexy-svgo-ffi)**
- **Status**: Using default features (`default` = `"clap"`)
- **Current Usage**: Build dependency for generating C headers
- **Issue**: "clap" feature is not needed - cbindgen is only used in build scripts
- **Recommendation**: Disable default features
- **Action Required**: Change `cbindgen = "0.26.0"` to `cbindgen = { version = "0.26.0", default-features = false }`

#### 3. **getrandom (Dual Versions)**
- **Status**: Both v0.2 and v0.3 are present
- **Usage**:
  - `getrandom_02` (v0.2): Used by wasm crate with features `["js", "wasm-bindgen"]`
  - `getrandom` (v0.3): Used by wasm crate with features `["wasm_js"]`
- **Recommendation**: Keep both for MSRV 1.74 compatibility (v0.2 may be required by some dependencies)

#### 4. **Other Dependencies (Status: Good)**
- **serde**: Workspace specifies `features = ["derive"]` - optimal
- **regex**: Using default features (std, perf, unicode) - necessary for performance
- **indexmap**: v2.10 with `std` default - appropriate
- **clap**: Workspace specifies `features = ["derive"]` - appropriate
- **once_cell**: Using default features - OK

### Workspace Feature Flags

#### vexy-svgo-core
```toml
[features]
default = []
json-schema = ["dep:jsonschema"]
parallel = ["dep:rayon"]
lyon = []
experimental = []
memory-profiling = []
data-uri = []
```

#### vexy-svgo-cli
- No feature flags defined (directly enables `json-schema` on core)

#### vexy-svgo-wasm
```toml
[features]
default = ["size-optimization"]
size-optimization = ["dep:wee_alloc"]
wasm = ["size-optimization"]
wasm-default = ["size-optimization"]
wasm-js = ["size-optimization"]
wasm-minimal = ["size-optimization"]
wasm-full = ["size-optimization"]
console_error_panic_hook = []
```

### Dependency Count Analysis

Heavy transitive dependencies brought in by optional features:
- **jsonschema**: 45+ transitive dependencies (reqwest ecosystem)
- **cbindgen**: 6 transitive dependencies (clap v3, indexmap v1, etc.)

## Recommended Changes

### 1. Fix cbindgen features
**File**: `crates/ffi/Cargo.toml`

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
cbindgen = { version = "0.26.0", default-features = false, features = [] }

[build-dependencies]
cbindgen = { version = "0.26.0", default-features = false, features = [] }
```

**Expected Impact**: Remove ~6 transitive dependencies (clap v3, ansi_term, atty, strsim, textwrap, etc.)

### 2. Document feature flags
Create `FEATURES.md` documenting all feature flags and their use cases.

## Feature Documentation

### json-schema (Core)
- **Purpose**: Validate SVGO configuration against JSON schema
- **Enabled by**: CLI crate
- **Dependencies**: jsonschema, reqwest, tokio
- **Use case**: Ensures configuration files follow SVGO specification
- **Weight**: Heavy (45+ transitive deps)
- **Recommendation**: Make truly optional - allow CLI to work without it (fallback to basic validation)

### parallel (Core)
- **Purpose**: Enable parallel optimization using rayon
- **Status**: Available but not currently enabled by default
- **Dependencies**: rayon
- **Use case**: Process multiple files/documents concurrently
- **Weight**: Light (rayon is well-optimized)

### size-optimization (WASM)
- **Purpose**: Use wee_alloc allocator for smaller WASM binary
- **Dependencies**: wee_alloc
- **Use case**: Reduce WASM bundle size
- **Weight**: Light

## Verification Steps

1. Apply the cbindgen fix
2. Run `cargo tree --workspace` to verify dependency reduction
3. Run `cargo test --workspace` to ensure all 877 tests pass
4. Run `cargo build --release` to verify release build succeeds
5. Compare binary sizes before/after

## Notes

- The project has been well-structured with feature flags already in place
- Most dependencies are using appropriate features
- The main optimization opportunity is removing the "clap" feature from cbindgen
- jsonschema is already properly gated, so it's only pulled in when the feature is enabled