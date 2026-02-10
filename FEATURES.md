# Feature Flags

Vexy SVGO provides optional feature flags to control which components and dependencies are compiled into the binary.

## Core Features (`vexy-svgo-core`)

### json-schema
**Enabled by default**: No (optional)

**Purpose**: Validate SVGO configuration files against the official SVGO JSON schema.

**Dependencies**: Adds `jsonschema` crate and its transitive dependencies (reqwest, tokio, hyper, etc.)

**Usage**:
```toml
# In Cargo.toml
[dependencies]
vexy-svgo-core = { version = "1.0", features = ["json-schema"] }

# Or via CLI
cargo build --features json-schema
```

**When to use**:
- You want strict validation of SVGO configuration files
- You need to ensure configurations follow the SVGO specification
- Building the full CLI with config validation

**When to skip**:
- You trust your configuration files or validate them separately
- You want a smaller binary and dependency tree
- Building for WASM where HTTP clients add overhead

**Trade-offs**:
- ✅ Strict validation ensures compatibility with SVGO
- ❌ Adds ~45 transitive dependencies and ~2MB to binary size

### parallel
**Enabled by default**: No (optional)

**Purpose**: Enable parallel optimization of multiple documents using rayon.

**Dependencies**: Adds `rayon` crate

**Usage**:
```toml
# In Cargo.toml
[dependencies]
vexy-svgo-core = { version = "1.0", features = ["parallel"] }
```

**When to use**:
- Processing multiple SVG files or documents concurrently
- Batch operations on large collections
- You want to leverage multi-core CPUs

**When to skip**:
- Processing single documents
- Building for platforms where parallelism isn't beneficial

**Trade-offs**:
- ✅ Significant speedup for batch operations (up to 8x on multi-core)
- ❌ Adds minimal overhead to binary size (~200KB)

### lyon
**Enabled by default**: No (optional placeholder)

**Purpose**: Integration with lyon path algorithms (currently a placeholder for future optimizations).

**Status**: Feature flag exists but code infrastructure is pending.

### experimental
**Enabled by default**: No (placeholder)

**Purpose**: Enable experimental optimizations that may be unstable or incomplete.

**Status**: Feature flag exists but code infrastructure is pending.

### memory-profiling
**Enabled by default**: No (placeholder)

**Purpose**: Enable memory profiling capabilities for development and debugging.

**Status**: Feature flag exists but code infrastructure is pending.

### data-uri
**Enabled by default**: No (placeholder)

**Purpose**: Enable Data URI encoding/decoding optimizations.

**Status**: Feature flag exists but code infrastructure is pending.

---

## WASM Features (`vexy-svgo-wasm`)

### size-optimization
**Enabled by default**: Yes (default feature)

**Purpose**: Use `wee_alloc` allocator to reduce WASM binary size.

**Dependencies**: Adds `wee_alloc` crate

**Usage**:
```toml
# Enable explicitly (already default)
[dependencies]
vexy-svgo-wasm = { version = "1.0", features = ["size-optimization"] }
```

**When to use**:
- Building WASM for production
- Binary size is a concern
- Standard library allocator can be larger

**When to skip**:
- Building for development
- You need the standard allocator for better heap performance
- Targeting environments where wee_alloc is not yet optimized

**Trade-offs**:
- ✅ Reduces WASM bundle size by ~10-20KB
- ❌ Slightly slower heap operations than standard allocator

### wasm
**Alias for**: `size-optimization`

### wasm-default
**Alias for**: `size-optimization`

### wasm-js
**Alias for**: `size-optimization`

### wasm-minimal
**Alias for**: `size-optimization`

### wasm-full
**Alias for**: `size-optimization`

### console_error_panic_hook
**Enabled by default**: No (optional)

**Purpose**: Provide better error messages in browser console when panics occur.

**Usage**:
```toml
[dependencies]
vexy-svgo-wasm = { version = "1.0", features = ["console_error_panic_hook"] }
```

**When to use**:
- Building WASM for development/debugging
- Want detailed panic messages in browser console

**When to skip**:
- Production builds where panic messages aren't visible anyway

---

## Recommended Configurations

### Minimal CLI (Linux/macOS/Windows)
```toml
[dependencies]
vexy-svgo-core = { version = "1.0", features = [] }
vexy-svgo-cli = { version = "1.0" }
```

### Full CLI with Config Validation
```toml
[dependencies]
vexy-svgo-core = { version = "1.0", features = ["json-schema"] }
vexy-svgo-cli = { version = "1.0" }
```

### Parallel Processing CLI
```toml
[dependencies]
vexy-svgo-core = { version = "1.0", features = ["parallel"] }
vexy-svgo-cli = { version = "1.0" }
```

### WASM Production Build
```toml
[dependencies]
vexy-svgo-wasm = { version = "1.0", features = ["size-optimization"] }
```

### WASM Development Build
```toml
[dependencies]
vexy-svgo-wasm = { version = "1.0", features = ["console_error_panic_hook"] }
```

---

## Dependency Impact Summary

| Feature | Added Dependencies | Binary Size Impact | Build Time Impact |
|---------|-------------------|-------------------|-------------------|
| json-schema | ~45 transitive | +2MB | +15% |
| parallel | 1 (rayon) | +200KB | +2% |
| size-optimization | 1 (wee_alloc) | -15KB (WASM) | <1% |
| console_error_panic_hook | 1 | +5KB (WASM) | <1% |

---

## Build Commands

```bash
# Build with default features
cargo build --release

# Build with specific features
cargo build --release --features json-schema,parallel

# Build WASM with optimization
wasm-pack build --features size-optimization

# Check which features are enabled
cargo tree --features
```

---

## Notes

- All features are additive; you can combine them as needed
- The `default` feature set is intentionally minimal
- Optional features are designed to not break the core functionality when disabled
- See `FEATURE_AUDIT.md` for detailed dependency analysis