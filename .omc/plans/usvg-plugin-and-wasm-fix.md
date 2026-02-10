# Work Plan: usvg Plugin Integration + WASM Build Fix

**Created**: 2026-02-09  
**Status**: Ready for execution  
**Priority**: High (TODO items for release)

---

## Context

### Original Request
Implement two TODO items for vexy-svgo:
1. **TODO 1**: Create a `usvg` plugin that normalizes SVG via usvg library
2. **TODO 2**: Fix WASM build failure caused by getrandom RUSTFLAGS override

### Interview Summary
| Question | Answer |
|----------|--------|
| Task ordering | Equal priority, parallel. WASM fix completes faster |
| Error handling | Graceful degradation: log warning, pass through original Document |
| Plugin ordering | Documentation-based: tell users to place usvg first in config |
| Test depth | Standard: config parsing + fixture tests with expected output |
| WASM verification | Just verify build succeeds |

### Research Findings
- **Parser API**: `vexy_svgo_core::parse_svg(input: &str) -> Result<Document<'static>, VexyError>`
- **Stringifier API**: `vexy_svgo_core::stringify(document: &Document) -> Result<String, VexyError>`
- **Plugin trait**: Located at `crates/core/src/plugin_registry.rs`
- **usvg 0.45.1** already in `[workspace.dependencies]` in root `Cargo.toml`
- **Current RUSTFLAGS issue**: `build-wasm.sh:26` and `scripts/build-wasm-optimized.sh:39` override `.cargo/config.toml`

---

## Work Objectives

### Core Objective
Deliver two independent, production-ready features:
1. A new `usvg` plugin that pre-processes SVG through usvg for normalization
2. A working WASM build by fixing RUSTFLAGS configuration

### Deliverables
| # | Deliverable | Type | Acceptance |
|---|-------------|------|------------|
| 1 | `crates/plugin-sdk/src/plugins/usvg.rs` | New file | Compiles, tests pass |
| 2 | Plugin registered in registry | Edit | `registry.create_plugin("usvg")` returns Some |
| 3 | Plugin exported in mod.rs | Edit | `use vexy_svgo_plugin_sdk::plugins::UsvgPlugin` works |
| 4 | usvg dependency in plugin-sdk | Edit | `cargo build` succeeds |
| 5 | Fixture tests for usvg plugin | New file | At least 2 test cases pass |
| 6 | `build-wasm.sh` fix | Edit | `./build-wasm.sh` succeeds |
| 7 | `scripts/build-wasm-optimized.sh` fix | Edit | `./scripts/build-wasm-optimized.sh` succeeds |

### Definition of Done
- [ ] All 515+ existing tests still pass (`cargo test --workspace`)
- [ ] New usvg plugin tests pass
- [ ] WASM build succeeds without errors
- [ ] No clippy warnings (strict clippy: unwrap/expect/panic/todo denied)
- [ ] Documentation comments complete

---

## Guardrails

### MUST Have
- Graceful error handling (log + pass-through, never panic)
- `#[serde(rename_all = "camelCase", deny_unknown_fields)]` on config structs
- Follow existing plugin file pattern exactly (see `remove_comments.rs`)
- `this_file:` comment at top of new files
- Unit tests for config parsing

### MUST NOT Have
- `unwrap()`, `expect()`, `panic!()`, `todo!()` in production code
- usvg in the default preset (it's opt-in only)
- Any changes to existing plugin behavior
- Force pushes or history rewrites

---

## Task Flow & Dependencies

```
[WASM Fix - Task 2]                [usvg Plugin - Task 1]
       |                                    |
  Edit RUSTFLAGS                    Add usvg to Cargo.toml
       |                                    |
  Build WASM                        Create usvg.rs plugin
       |                                    |
  Verify success                    Edit mod.rs (pub mod + pub use)
       |                                    |
       +-----> DONE                 Edit registry.rs (register call)
                                            |
                                    Add to get_migrated_plugin_names()
                                            |
                                    Write unit tests
                                            |
                                    Write fixture tests
                                            |
                                    Run cargo test
                                            |
                                            +-----> DONE
```

Both tasks are **fully independent** and can run in parallel.

---

## Detailed TODOs

### TASK 1: usvg Plugin Integration

#### TODO 1.1: Add usvg dependency to plugin-sdk
**File**: `crates/plugin-sdk/Cargo.toml`  
**Location**: After line 83 (after `lyon`)  
**Change**: Add usvg dependency

```toml
[dependencies.usvg]
workspace = true
```

**Acceptance**: `cargo build -p vexy-svgo-plugin-sdk` succeeds

---

#### TODO 1.2: Create the usvg plugin file
**File**: `crates/plugin-sdk/src/plugins/usvg.rs` (NEW)  
**Pattern**: Follow `remove_comments.rs` exactly

**Config struct** (with serde attributes):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UsvgConfig {
    #[serde(default = "default_dpi")]
    pub dpi: f32,
    
    #[serde(default = "default_coordinates_precision")]
    pub coordinates_precision: u8,
    
    #[serde(default = "default_transforms_precision")]
    pub transforms_precision: u8,
}

fn default_dpi() -> f32 { 96.0 }
fn default_coordinates_precision() -> u8 { 3 }
fn default_transforms_precision() -> u8 { 4 }
```

**Plugin struct**:
```rust
#[derive(Clone)]
pub struct UsvgPlugin {
    config: UsvgConfig,
}
```

**Key implementation in apply()**:
1. Stringify: `vexy_svgo_core::stringify(document)?` -> get SVG string
2. usvg process: `usvg::Tree::from_str()` with Options from config
3. usvg output: `tree.to_string()` with WriteOptions from config
4. Re-parse: `vexy_svgo_core::parse_svg(&usvg_output)?`
5. Replace document contents: copy root, prologue, epilogue from new doc
6. Error handling: on ANY error, log with `eprintln!` and return `Ok(())`

**Error handling pattern**:
```rust
fn apply<'a>(&self, document: &mut Document<'a>) -> Result<()> {
    // Step 1: Stringify current document
    let svg_string = match vexy_svgo_core::stringify(document) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[usvg] Warning: Failed to stringify document: {}. Passing through unchanged.", e);
            return Ok(());
        }
    };
    
    // Step 2: Process with usvg
    let options = self.build_usvg_options();
    let tree = match usvg::Tree::from_str(&svg_string, &options) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[usvg] Warning: Failed to parse with usvg: {}. Passing through unchanged.", e);
            return Ok(());
        }
    };
    
    // Step 3: Convert back to string
    let write_options = self.build_write_options();
    let usvg_output = tree.to_string(&write_options);
    
    // Step 4: Re-parse into Document
    let new_doc = match vexy_svgo_core::parse_svg(&usvg_output) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[usvg] Warning: Failed to re-parse usvg output: {}. Passing through unchanged.", e);
            return Ok(());
        }
    };
    
    // Step 5: Replace document contents
    document.root = new_doc.root;
    document.prologue = new_doc.prologue;
    document.epilogue = new_doc.epilogue;
    document.metadata = new_doc.metadata;
    
    Ok(())
}
```

**Acceptance**: File compiles with no warnings

---

#### TODO 1.3: Export plugin in mod.rs
**File**: `crates/plugin-sdk/src/plugins/mod.rs`  
**Changes**:
1. Add `pub mod usvg;` (line ~63, alphabetically after `sort_defs_children`)
2. Add `pub use usvg::UsvgPlugin;` (line ~119, alphabetically)

**Acceptance**: `use vexy_svgo_plugin_sdk::plugins::UsvgPlugin;` compiles

---

#### TODO 1.4: Register plugin in registry
**File**: `crates/plugin-sdk/src/registry.rs`  
**Change 1**: Add registration (after line 70, before `registry` return):
```rust
registry.register("usvg", || UsvgPlugin::new());
```

**Change 2**: Add to `get_migrated_plugin_names()` (after line 386):
```rust
"usvg",
```

**Note**: Do NOT add to `get_default_plugin_configs()` - usvg is opt-in only

**Acceptance**: 
- `registry.create_plugin("usvg").is_some()` returns true
- `get_migrated_plugin_names().contains(&"usvg")` returns true

---

#### TODO 1.5: Write unit tests
**File**: `crates/plugin-sdk/src/plugins/usvg.rs` (bottom of file)

**Tests to include**:
1. `test_plugin_info()` - verify name() and description()
2. `test_plugin_creation()` - verify default and custom config
3. `test_parameter_validation()` - valid/invalid params
4. `test_config_parsing()` - JSON to config struct
5. `test_default_config_values()` - verify defaults (dpi=96, etc.)

**Pattern**: Copy test structure from `remove_comments.rs`

**Acceptance**: All unit tests pass

---

#### TODO 1.6: Create fixture test
**Directory**: `testdata/plugins/usvg/` (NEW)

**Files to create**:
1. `simple.svg` - input SVG with some complexity (use statements, defs, etc.)
2. `simple.svg.expected` - expected output after usvg processing

**Example input** (`simple.svg`):
```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <defs>
    <linearGradient id="grad1">
      <stop offset="0%" style="stop-color:rgb(255,255,0);stop-opacity:1" />
      <stop offset="100%" style="stop-color:rgb(255,0,0);stop-opacity:1" />
    </linearGradient>
  </defs>
  <rect fill="url(#grad1)" x="10" y="10" width="80" height="80"/>
</svg>
```

**Note**: Expected output comes from running actual usvg on the input. The fixture test macro `plugin_fixture_tests!` handles comparison.

**Acceptance**: `cargo test -p vexy-svgo-plugin-sdk usvg` passes

---

#### TODO 1.7: Add fixture test invocation
**File**: `crates/plugin-sdk/src/plugins/usvg.rs` (end of file)

```rust
// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests!(UsvgPlugin, "usvg");
```

**Acceptance**: Fixture tests run and pass

---

### TASK 2: Fix WASM Build

#### TODO 2.1: Fix build-wasm.sh RUSTFLAGS
**File**: `build-wasm.sh`  
**Line**: 26  
**Current**:
```bash
RUSTFLAGS="-C opt-level=z -C lto=fat" wasm-pack build ...
```

**Change to**:
```bash
RUSTFLAGS="-C opt-level=z -C lto=fat --cfg getrandom_backend=\"wasm_js\"" wasm-pack build ...
```

**Acceptance**: `./build-wasm.sh` completes without getrandom errors

---

#### TODO 2.2: Fix scripts/build-wasm-optimized.sh RUSTFLAGS
**File**: `scripts/build-wasm-optimized.sh`  
**Line**: 39  
**Current**:
```bash
RUSTFLAGS="-C opt-level=z -C lto=fat -C embed-bitcode=yes" \
```

**Change to**:
```bash
RUSTFLAGS="-C opt-level=z -C lto=fat -C embed-bitcode=yes --cfg getrandom_backend=\"wasm_js\"" \
```

**Acceptance**: `./scripts/build-wasm-optimized.sh` completes without getrandom errors

---

#### TODO 2.3: Verify WASM build
**Command**: `./build-wasm.sh`

**Expected outcome**:
- No compilation errors
- `pkg-web/`, `pkg-node/`, `pkg-bundler/`, `pkg-minimal/`, `pkg-full/` directories created
- `.wasm` files present in each directory

**Acceptance**: Build script exits with code 0

---

## Commit Strategy

### Commit 1: Fix WASM build (Task 2)
```
fix(wasm): add getrandom_backend cfg to RUSTFLAGS in build scripts

The build scripts were overriding .cargo/config.toml RUSTFLAGS,
causing getrandom 0.3.3 to fail for wasm32-unknown-unknown target.

Fixes #125
```

**Files**:
- `build-wasm.sh`
- `scripts/build-wasm-optimized.sh`

### Commit 2: Add usvg plugin (Task 1)
```
feat(plugin): add usvg plugin for SVG normalization

Adds a new opt-in plugin that pre-processes SVG through usvg library.
This normalizes and simplifies SVG before other optimizations run.

Config options:
- dpi: f32 (default 96.0)
- coordinatesPrecision: u8 (default 3)
- transformsPrecision: u8 (default 4)

The plugin uses graceful degradation - on any error, it logs a
warning and passes through the original document unchanged.

Users should place "usvg" first in their plugin config array.
```

**Files**:
- `crates/plugin-sdk/Cargo.toml`
- `crates/plugin-sdk/src/plugins/mod.rs`
- `crates/plugin-sdk/src/plugins/usvg.rs` (NEW)
- `crates/plugin-sdk/src/registry.rs`
- `testdata/plugins/usvg/simple.svg` (NEW)
- `testdata/plugins/usvg/simple.svg.expected` (NEW)

---

## Success Criteria

### Task 1: usvg Plugin
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test -p vexy-svgo-plugin-sdk usvg` passes
- [ ] `cargo clippy --workspace` has no warnings
- [ ] Plugin correctly handles errors with graceful degradation
- [ ] Config camelCase works (dpi, coordinatesPrecision, transformsPrecision)

### Task 2: WASM Fix
- [ ] `./build-wasm.sh` exits with code 0
- [ ] `./scripts/build-wasm-optimized.sh` exits with code 0
- [ ] `.wasm` files are generated in pkg directories

### Overall
- [ ] All 515+ existing tests pass
- [ ] No regressions in other plugins
- [ ] Both features work independently

---

## Delegation Recommendations

| Task | Agent/Skill | Rationale |
|------|-------------|-----------|
| TODO 1.1-1.4 | Hephaestus (builder) | Standard Rust implementation |
| TODO 1.5-1.7 | Hephaestus (builder) | Test writing |
| TODO 2.1-2.3 | Quick task | Trivial shell script edits |

**Recommended execution order**:
1. Start WASM fix first (trivial, fast feedback)
2. Work on usvg plugin in parallel
3. Run full test suite after both complete

---

## Appendix: Reference Code

### usvg API Reference (v0.45.1)
```rust
// Parse SVG into usvg Tree
let options = usvg::Options {
    dpi: 96.0,
    ..Default::default()
};
let tree = usvg::Tree::from_str(svg_str, &options)?;

// Convert back to string
let write_options = usvg::WriteOptions {
    coordinates_precision: 3,
    transforms_precision: 4,
    ..Default::default()
};
let output = tree.to_string(&write_options);
```

### Existing Plugin Pattern (remove_comments.rs)
- Lines 1-14: File header, imports
- Lines 16-35: Config struct with serde + defaults
- Lines 37-118: Plugin struct + methods
- Lines 120-146: Plugin trait impl
- Lines 148-453: Tests
- Line 456: Fixture test macro
