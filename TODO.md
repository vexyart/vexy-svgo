---
this_file: TODO.md
---

ULTRAWORK: Deploy a team of 3 agents: Teammate 1 is the Supervisor who analyzes, plans, coordinates, orchestrates the next steps & delegates the work; Teammate 2 is the Coder who implements changes in the code; Teammate 3 is the Tester who examines, tests, verifies, debugs, and who reports to Supervisor; repeat until all tasks are completed & the app is highly polished & well-developed. 

# Vexy SVGO Priority Roadmap (Reconciled)

This file now contains only unresolved roadmap items.
Verified completed items were moved to `CHANGELOG.md` on 2026-02-10.

## Wave 1: Foundation & Performance (Core Blockers)
*Focus: Eliminating architectural bottlenecks and establishing infrastructure.*

- [ ] [Core] AST Arena Allocation: Implement a typed arena allocator (bumpalo) for the document AST. | **Skill: Rust-Memory**
- [ ] [Core] CSS Variable Resolution: Integrate scoped CSS variable resolver into optimization passes (not only standalone module/tests). | **Skill: Rust-Core**
- [ ] [Infra] Feature-Gated Dependencies: Audit and minimize default features across the workspace. | **Skill: Rust-Maintenance**
- [ ] [Infra] MSRV Enforcement: Align declared MSRV target and CI enforcement (current CI/toolchain mismatch with roadmap target). | **Skill: CI-CD**
- [ ] [Core] Robust Fuzzing Suite: Integrate cargo-fuzz to test against malformed XML inputs. | **Skill: Rust-Security**
- [ ] [Docs] API Reference: Host full rustdoc documentation for core and plugin-sdk. | **Skill: Technical-Writing**
- [ ] [Core] Memory Budgeting: Implement optimizer-wide runtime memory budget (parser-only budgeting is not sufficient). | **Skill: Rust-Core**

## Wave 2: Intelligence & Advanced Plugins
*Focus: Advanced SVG optimization logic and stabilization.*

- [ ] [Plugin] mergePaths Spatial Index: Integrate an R-tree for spatial indexing to speed up intersection checks. | **Skill: Rust-Algorithms**
- [ ] [Core] Parallel Node Processing: Enable real concurrent plugin application via rayon (current element-category path falls back to sequential). | **Skill: Rust-Concurrency**
- [ ] [Core] Fast-Path Path Parser: Add a specialized SIMD-accelerated parser for the `d` attribute. | **Skill: Rust-SIMD**
- [ ] [Plugin] convertPathData Arc-to-Bezier: Implement high-precision conversion of `A` commands to cubic Beziers. | **Skill: Graphics-Math**
- [ ] [Plugin] convertPathData Colinear Line Merging: Add logic to detect and merge consecutive colinear line segments. | **Skill: Graphics-Math**
- [ ] [CI] Performance Regression Check: Fail PRs that increase average processing time by >5%. | **Skill: CI-CD**
- [ ] [CI] Visual Regression Testing: Headless browser-based rendering comparison suite. | **Skill: CI-CD**
- [ ] [Plugin] inlineStyles Selective Inlining: Add option to selectively inline specific elements/classes. | **Skill: Rust-Core**

## Wave 3: Ecosystem, DX & Bindings
*Focus: Distribution, tooling, and multi-language support.*

- [ ] [WASM] Size-Optimized Bundle: Keep aggressive wasm-opt pipelines and verify/track `<80KB` target in CI artifacts. | **Skill: WebAssembly**
- [ ] [Bindings] Python pyo3 Wrapper: Native Python bindings for pip installation. | **Skill: Rust-FFI**
- [ ] [Bindings] Go/C-Shared Library: Stabilize the FFI layer for universal language support (including language-level examples/tooling). | **Skill: Rust-FFI**
- [ ] [CLI] Interactive Mode: Implement an interactive terminal UI for plugin selection. | **Skill: Rust-TUI**
- [ ] [CLI] Structural Diff Mode: Add a `--diff` flag for color-coded structural diff output. | **Skill: Rust-CLI**
- [ ] [CLI] Watch Mode: Implement a file-watcher for automatic re-optimization on save. | **Skill: Rust-Core**
- [ ] [CLI] Batch Statistics Report: Provide a summary table showing total bytes saved across directories. | **Skill: Rust-CLI**
- [ ] [WASM] React/Vue Component Wrapper: Create high-level wrappers for popular frontend frameworks. | **Skill: Frontend-UI-UX**
- [ ] [Tooling] VS Code Extension: Develop a basic extension for SVG optimization on save. | **Skill: VS-Code-Ext**
- [ ] [Tooling] Benchmarking Dashboard: Create an HTML/JS dashboard to visualize performance over time. | **Skill: Frontend-UI-UX**
- [ ] [Ecosystem] GitHub Action: Create an official Action for SVG optimization in CI. | **Skill: GitHub-Actions**
