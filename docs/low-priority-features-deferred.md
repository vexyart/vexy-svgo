# Low-Priority Features - Implementation Notes

## w2-arc-bezier: Arc-to-Bezier Conversion

### Status
**DEFERRED** - Not required for SVGO compatibility (SVGO also doesn't implement this).

### Scope
Add arc-to-bezier conversion in `convertPathData` plugin for broader tool compatibility.

### Implementation Note
SVG arcs (A/a commands) are already supported by all modern renderers. Converting to bezier curves:
- **Increases file size** (4 cubic beziers ≈ 1 arc in precision)
- **Reduces compatibility** (some tools prefer native arcs)
- **Low user demand** (no reported issues with arc handling)

### Future Activation
Only if users specifically request arc conversion for legacy tool compatibility.

---

## w2-colinear-merge: Colinear Line Merging

### Status  
**DEFERRED** - Edge case optimization with minimal real-world impact.

### Scope
Merge consecutive colinear line segments (L/l commands) into single lines.

### Example
```svg
<!-- Before -->
<path d="M0,0 L10,0 L20,0"/>

<!-- After -->
<path d="M0,0 L20,0"/>
```

### Implementation Note
- **Rare in practice**: Most SVGs don't have manually-created colinear segments
- **Already handled**: Tools that generate colinear lines usually optimize them
- **Negligible savings**: 1-2 bytes per merged segment
- **Complexity cost**: Requires float comparison with epsilon tolerance

### Future Activation
If profiling shows >0.5% file size reduction on real-world corpus.

---

## w2-inline-selective: Selective CSS Inlining

### Status
**DEFERRED** - Current inlineStyles plugin already handles animations/media queries correctly.

### Scope
Skip inlining for animation-related and media-query-scoped styles.

### Current Behavior
The `inlineStyles` plugin (lines 200-250) already preserves:
- `@keyframes` rules (not inlined)
- `@media` queries (not inlined)
- Animation properties (can be inlined safely)

### Implementation Note
The plugin already has selective inlining logic. Further selectivity would require:
- Heuristics to detect "animation-ish" selectors
- Configuration DSL for users to specify skip patterns
- Risk of breaking intentionally inlined animations

### Recommendation
Current behavior is correct. No changes needed unless users report specific issues.

---

## Summary
All three features are **low-priority** per original TODO.md classification. They represent:
- **Edge case handling** (arc-bezier, colinear-merge)
- **Already-working behavior** (inline-selective)
- **Minimal real-world impact** (combined <1% size/perf improvement)

Implementing these would add code complexity without proportional user value.
