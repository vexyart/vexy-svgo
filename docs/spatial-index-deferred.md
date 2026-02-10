# Spatial Index (R-tree) for mergePaths - Implementation Note

## Status
**DEFERRED** - Current O(n²) AABB checking is sufficient for typical SVG sizes.

## Current Performance
The `merge_paths.rs` plugin uses nested loops (lines 69-78) to check path intersections:
```rust
for p1 in &paths1 {
    for p2 in &paths2 {
        if bboxes_overlap(&p1.bbox, &p2.bbox) {
            possible_intersection = true;
            break;
        }
    }
}
```

## Proposed Optimization
Add `rstar` R-tree crate for O(log n) spatial queries when path count exceeds threshold (>100 paths).

## Implementation Sketch
```rust
#[cfg(feature = "spatial-index")]
use rstar::RTree;

fn find_overlapping_paths_rtree(paths: &[PathData]) -> Vec<(usize, usize)> {
    let tree: RTree<BBoxEntry> = RTree::bulk_load(
        paths.iter().enumerate()
            .map(|(i, p)| BBoxEntry { idx: i, bbox: p.bbox })
            .collect()
    );
    
    let mut overlaps = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        for entry in tree.locate_in_envelope(&path.bbox.to_envelope()) {
            if entry.idx > i { // Avoid duplicates
                overlaps.push((i, entry.idx));
            }
        }
    }
    overlaps
}
```

## Benchmark Target
- Current: O(n²) = ~10,000 comparisons for 100 paths
- R-tree: O(n log n) = ~664 comparisons for 100 paths
- **Expected speedup**: 15x for SVGs with 100+ paths

## Why Deferred
1. Typical SVGs have <50 paths (O(n²) is fast enough)
2. AABB pre-filtering already eliminates most comparisons
3. Adds dependency and complexity for marginal real-world gain
4. No user complaints about mergePaths performance

## Future Activation
Enable via feature flag when profiling shows mergePaths is a bottleneck:
```toml
[features]
spatial-index = ["rstar"]
```
