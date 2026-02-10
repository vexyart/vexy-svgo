---
this_file: docs_src/developer/compatibility-matrix.md
nav_weight: 36
title: SVGO Compatibility Matrix
---

# SVGO v3 Compatibility Matrix

This matrix tracks plugin availability and known output/behaviour differences between **Vexy SVGO** and **SVGO v3.x**.

## Summary

| Area | Status |
|---|---|
| Plugin names and config format compatibility | High |
| Default preset parity (core SVGO defaults) | High |
| Byte-for-byte output parity across all fixtures | Partial |
| CLI flag parity with advanced SVGO workflows | Partial |

## Plugin Support Matrix

Legend:

- ✅ Implemented and registered in plugin registry
- ⚠️ Implemented with known behavioural/output differences in some fixtures
- ❌ Not implemented

| Category | Plugin | Status | Notes |
|---|---|---|---|
| Core/default | `removeComments` | ✅ | Supports `preservePatterns` |
| Core/default | `cleanupAttrs` | ✅ | Namespace cleanup included |
| Core/default | `mergeStyles` | ✅ | Fixture-aligned whitespace handling |
| Core/default | `inlineStyles` | ⚠️ | Implemented; selective inlining controls still pending |
| Core/default | `minifyStyles` | ✅ | Includes usage-aware filtering |
| Core/default | `cleanupIds` | ✅ | Includes `preserve`/`preservePrefixes`; global `prefix` option added |
| Core/default | `convertPathData` | ⚠️ | Advanced path transforms implemented; exact formatting may differ |
| Core/default | `convertTransform` | ✅ | Registered and configurable |
| Core/default | `mergePaths` | ⚠️ | Implemented; spatial index acceleration pending |
| Core/default | `sortDefsChildren` | ✅ | Compression-oriented ordering |
| Non-default | `removeDimensions` | ✅ | Responsive `preserveAspectRatio` injection supported |
| Non-default | `prefixIds` | ✅ | Prefixing IDs/classes/references |
| Non-default | `reusePaths` | ✅ | Registered in plugin SDK registry |
| Non-default | `removeAttributesBySelector` | ✅ | Selector-based removal supported |
| Vexy extension | `usvg` | ✅ | Non-SVGO plugin for normalization |

## Known Differences vs SVGO v3

| Topic | Current state |
|---|---|
| Path command serialization | May differ while remaining semantically equivalent |
| ID minification naming sequence | May differ from SVGO's exact chosen symbols |
| Some fixture outputs | Minor whitespace/ordering differences may remain |
| CLI options | Several advanced flags remain unimplemented (`--watch`, `--diff`, etc.) |

## Verification Sources

- Plugin registration: `crates/plugin-sdk/src/registry.rs`
- Plugin implementations: `crates/plugin-sdk/src/plugins/`
- Compatibility tests and fixtures: `test/`, `testdata/plugins/`, crate-level tests
- Changelog verification notes: `CHANGELOG.md`

## How to Keep This Matrix Current

1. When adding or changing a plugin, update this table in the same PR.
2. If behaviour differs intentionally from SVGO, document it in "Known Differences".
3. If a pending item reaches parity, update status from ⚠️ to ✅ and link evidence.
