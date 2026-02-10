---
this_file: TASKS.md
---

# Task Batches (derived from TODO roadmap)

## Batch 1 (completed)

- [x] [Core] Streaming Stringifier: Complete writer-backed element/node serialization and parity tests.
- [x] [Plugin] cleanupIds Global Prefixing: Add `prefix` option and reference rewrite coverage.
- [x] [Plugin] removeDimensions Responsive Mode: Inject `preserveAspectRatio="xMidYMid meet"` when dimensions are removed.
- [x] [Examples] C FFI Usage: Add minimal C integration example and run instructions.
- [x] [Docs] Compatibility Matrix: Add SVGO v3 compatibility matrix page.

## Batch 2 (pending)

- [ ] [Core] AST Arena Allocation (`bumpalo`) for AST.
- [ ] [Core] CSS Variable Resolution integration into optimization pipeline.
- [ ] [Infra] Feature-gated dependency audit/minimization.
- [ ] [Infra] MSRV enforcement alignment (toolchain/workflows/docs).
- [ ] [Core] Robust fuzzing suite (`cargo-fuzz`).

## Batch 3 (pending)

- [ ] [Plugin] mergePaths spatial index (R-tree) acceleration.
- [ ] [Core] true concurrent plugin application via rayon.
- [ ] [Core] SIMD fast-path path parser.
- [ ] [Plugin] convertPathData Arc→Bezier conversion for `A` commands.
- [ ] [Plugin] convertPathData colinear line merge.

## Batch 4 (pending)

- [ ] [CI] Performance regression guard (>5% fail threshold).
- [ ] [CI] Visual regression testing suite.
- [ ] [Plugin] inlineStyles selective inlining options.
- [ ] [Docs] API reference hosting validation and publishing.
- [ ] [Core] optimizer-wide memory budgeting.

## Batch 5 (pending)

- [ ] [Bindings] Python `pyo3` wrapper.
- [ ] [Bindings] Go/C-shared layer stabilization and language-level examples.
- [ ] [CLI] Interactive mode.
- [ ] [CLI] Structural diff mode (`--diff`).
- [ ] [CLI] Watch mode.
- [ ] [CLI] Batch statistics report.

## Batch 6 (pending)

- [ ] [WASM] <80KB target enforcement and CI tracking.
- [ ] [WASM] React/Vue wrappers.
- [ ] [Tooling] VS Code extension.
- [ ] [Ecosystem] GitHub Action.
