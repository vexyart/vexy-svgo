// this_file: crates/plugin-sdk/src/plugins/merge_paths.rs

//! Merge paths plugin implementation
//!
//! This plugin merges multiple path elements that can be combined into a single path.
//! This is a complex optimization that can significantly reduce SVG file size.
//!
//! Note: This is a basic implementation focused on simple cases. A full implementation
//! would require advanced path analysis using the lyon crate for proper path operations
//! and sophisticated intersection detection.

use std::sync::LazyLock;

use anyhow::Result;
use regex::Regex;
use serde_json::Value;
use vexy_svgo_core::ast::{Document, Element, Node};
use vexy_svgo_core::error::VexyError;
use vexy_svgo_core::utils::paths::PathCommand;
use vexy_svgo_core::visitor::Visitor;

use crate::Plugin;

/// Plugin configuration for merge paths
#[derive(Debug, Clone)]
pub struct MergePathsConfig {
    pub force:                bool,
    pub float_precision:      u8,
    pub no_space_after_flags: bool,
}

impl Default for MergePathsConfig {
    fn default() -> Self {
        Self {
            force:                false,
            float_precision:      3,
            no_space_after_flags: false,
        }
    }
}

/// Plugin that merges compatible path elements
pub struct MergePathsPlugin {
    config: MergePathsConfig,
}

fn paths_intersect(d1: &str, d2: &str) -> bool {
    let (paths1, global1) = gather_path_points(d1);
    let (paths2, global2) = gather_path_points(d2);

    if paths1.is_empty() || paths2.is_empty() {
        return false;
    }

    // Quick global AABB pre-check
    if !bboxes_overlap(&global1, &global2) {
        return false;
    }

    // Per-subpath AABB check
    // If ANY pair of subpaths might overlap (based on AABB), we must check deeper.
    // SVGO logic:
    // 1. Check if global BBoxes overlap. If not, return false.
    // 2. Check if specific subpath BBoxes overlap. SVGO's `intersects` function returns false if:
    //    points1.list.every(set1 => points2.list.every(set2 => non-overlapping)) i.e. if ALL pairs are
    //    non-overlapping, return false. Equivalently: if ANY pair overlaps in AABB, we proceed to GJK.

    let mut possible_intersection = false;
    for p1 in &paths1 {
        for p2 in &paths2 {
            if bboxes_overlap(&p1.bbox, &p2.bbox) {
                possible_intersection = true;
                break;
            }
        }
        if possible_intersection {
            break;
        }
    }

    if !possible_intersection {
        return false;
    }

    // Convex Hull + GJK check
    // Compute hulls for all subpaths
    let hulls1: Vec<Vec<Point>> = paths1.iter().map(|p| convex_hull(&p.points)).collect();
    let hulls2: Vec<Vec<Point>> = paths2.iter().map(|p| convex_hull(&p.points)).collect();

    // Check intersection of every subpath hull of path1 with every subpath hull of path2
    for hull1 in &hulls1 {
        // Skip hulls with fewer than 3 points as they cannot form a polygon.
        // This matches SVGO behavior (see _path.js:272).
        if hull1.len() < 3 {
            continue;
        }

        for hull2 in &hulls2 {
            if hull2.len() < 3 {
                continue;
            }

            if gjk_intersects(hull1, hull2) {
                return true;
            }
        }
    }

    false
}

type Point = [f64; 2];

#[derive(Debug, Clone)]
struct BBox {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl BBox {
    fn new() -> Self {
        Self {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        }
    }

    fn update(&mut self, p: Point) {
        if p[0] < self.min_x {
            self.min_x = p[0];
        }
        if p[0] > self.max_x {
            self.max_x = p[0];
        }
        if p[1] < self.min_y {
            self.min_y = p[1];
        }
        if p[1] > self.max_y {
            self.max_y = p[1];
        }
    }

    fn is_valid(&self) -> bool {
        !self.min_x.is_infinite()
    }
}

struct SubPath {
    points: Vec<Point>,
    bbox:   BBox,
}

fn bboxes_overlap(a: &BBox, b: &BBox) -> bool {
    if !a.is_valid() || !b.is_valid() {
        return false;
    }
    !(a.max_x <= b.min_x || b.max_x <= a.min_x || a.max_y <= b.min_y || b.max_y <= a.min_y)
}

fn gather_path_points(d: &str) -> (Vec<SubPath>, BBox) {
    use vexy_svgo_core::utils::paths::PathUtils;

    let commands = PathUtils::parse_path_data(d);
    if commands.is_empty() {
        return (Vec::new(), BBox::new());
    }

    let mut subpaths: Vec<SubPath> = Vec::new();
    let mut global_bbox = BBox::new();

    // Current state
    let mut current_subpath_points: Vec<Point> = Vec::new();
    let mut current_subpath_bbox = BBox::new();

    let mut current_x = 0.0;
    let mut current_y = 0.0;
    let mut start_x = 0.0;
    let mut start_y = 0.0;
    let mut prev_control_x = 0.0;
    let mut prev_control_y = 0.0;
    let mut last_cmd_was_curve = false;
    let mut last_cmd_was_quad = false;

    // Helper to add point to current subpath
    let add_point = |p: Point, points: &mut Vec<Point>, bbox: &mut BBox, g_bbox: &mut BBox| {
        points.push(p);
        bbox.update(p);
        g_bbox.update(p);
    };

    // Flush current subpath
    let flush_subpath = |points: &mut Vec<Point>, bbox: &mut BBox, result: &mut Vec<SubPath>| {
        if !points.is_empty() {
            result.push(SubPath {
                points: std::mem::take(points),
                bbox:   std::mem::replace(bbox, BBox::new()),
            });
        }
    };

    let reflect = |p: (f64, f64), cp: (f64, f64)| -> (f64, f64) { (2.0 * p.0 - cp.0, 2.0 * p.1 - cp.1) };

    for cmd in commands {
        match cmd.command {
            'M' => {
                flush_subpath(&mut current_subpath_points, &mut current_subpath_bbox, &mut subpaths);

                if let (Some(&x), Some(&y)) = (cmd.params.get(0), cmd.params.get(1)) {
                    current_x = x;
                    current_y = y;
                    start_x = current_x;
                    start_y = current_y;

                    add_point(
                        [current_x, current_y],
                        &mut current_subpath_points,
                        &mut current_subpath_bbox,
                        &mut global_bbox,
                    );

                    let mut i = 2;
                    while i + 1 < cmd.params.len() {
                        current_x = cmd.params[i];
                        current_y = cmd.params[i + 1];
                        add_point(
                            [current_x, current_y],
                            &mut current_subpath_points,
                            &mut current_subpath_bbox,
                            &mut global_bbox,
                        );
                        i += 2;
                    }
                }
                last_cmd_was_curve = false;
                last_cmd_was_quad = false;
            },
            'm' => {
                flush_subpath(&mut current_subpath_points, &mut current_subpath_bbox, &mut subpaths);

                if let (Some(&dx), Some(&dy)) = (cmd.params.get(0), cmd.params.get(1)) {
                    current_x += dx;
                    current_y += dy;
                    start_x = current_x;
                    start_y = current_y;

                    add_point(
                        [current_x, current_y],
                        &mut current_subpath_points,
                        &mut current_subpath_bbox,
                        &mut global_bbox,
                    );

                    let mut i = 2;
                    while i + 1 < cmd.params.len() {
                        current_x += cmd.params[i];
                        current_y += cmd.params[i + 1];
                        add_point(
                            [current_x, current_y],
                            &mut current_subpath_points,
                            &mut current_subpath_bbox,
                            &mut global_bbox,
                        );
                        i += 2;
                    }
                }
                last_cmd_was_curve = false;
                last_cmd_was_quad = false;
            },
            'L' => {
                let mut i = 0;
                while i + 1 < cmd.params.len() {
                    current_x = cmd.params[i];
                    current_y = cmd.params[i + 1];
                    add_point(
                        [current_x, current_y],
                        &mut current_subpath_points,
                        &mut current_subpath_bbox,
                        &mut global_bbox,
                    );
                    i += 2;
                }
                last_cmd_was_curve = false;
                last_cmd_was_quad = false;
            },
            'l' => {
                let mut i = 0;
                while i + 1 < cmd.params.len() {
                    current_x += cmd.params[i];
                    current_y += cmd.params[i + 1];
                    add_point(
                        [current_x, current_y],
                        &mut current_subpath_points,
                        &mut current_subpath_bbox,
                        &mut global_bbox,
                    );
                    i += 2;
                }
                last_cmd_was_curve = false;
                last_cmd_was_quad = false;
            },
            'H' => {
                for &x in &cmd.params {
                    current_x = x;
                    add_point(
                        [current_x, current_y],
                        &mut current_subpath_points,
                        &mut current_subpath_bbox,
                        &mut global_bbox,
                    );
                }
                last_cmd_was_curve = false;
                last_cmd_was_quad = false;
            },
            'h' => {
                for &dx in &cmd.params {
                    current_x += dx;
                    add_point(
                        [current_x, current_y],
                        &mut current_subpath_points,
                        &mut current_subpath_bbox,
                        &mut global_bbox,
                    );
                }
                last_cmd_was_curve = false;
                last_cmd_was_quad = false;
            },
            'V' => {
                for &y in &cmd.params {
                    current_y = y;
                    add_point(
                        [current_x, current_y],
                        &mut current_subpath_points,
                        &mut current_subpath_bbox,
                        &mut global_bbox,
                    );
                }
                last_cmd_was_curve = false;
                last_cmd_was_quad = false;
            },
            'v' => {
                for &dy in &cmd.params {
                    current_y += dy;
                    add_point(
                        [current_x, current_y],
                        &mut current_subpath_points,
                        &mut current_subpath_bbox,
                        &mut global_bbox,
                    );
                }
                last_cmd_was_curve = false;
                last_cmd_was_quad = false;
            },
            'C' => {
                let mut i = 0;
                while i + 5 < cmd.params.len() {
                    let x1 = cmd.params[i];
                    let y1 = cmd.params[i + 1];
                    let x2 = cmd.params[i + 2];
                    let y2 = cmd.params[i + 3];
                    let x = cmd.params[i + 4];
                    let y = cmd.params[i + 5];

                    // SVGO Approximation: midpoints
                    add_point(
                        [0.5 * (current_x + x1), 0.5 * (current_y + y1)],
                        &mut current_subpath_points,
                        &mut current_subpath_bbox,
                        &mut global_bbox,
                    );
                    add_point(
                        [0.5 * (x1 + x2), 0.5 * (y1 + y2)],
                        &mut current_subpath_points,
                        &mut current_subpath_bbox,
                        &mut global_bbox,
                    );
                    add_point(
                        [0.5 * (x2 + x), 0.5 * (y2 + y)],
                        &mut current_subpath_points,
                        &mut current_subpath_bbox,
                        &mut global_bbox,
                    );
                    add_point([x, y], &mut current_subpath_points, &mut current_subpath_bbox, &mut global_bbox);

                    current_x = x;
                    current_y = y;
                    prev_control_x = x2;
                    prev_control_y = y2;
                    i += 6;
                }
                last_cmd_was_curve = true;
                last_cmd_was_quad = false;
            },
            'c' => {
                let mut i = 0;
                while i + 5 < cmd.params.len() {
                    let x1 = current_x + cmd.params[i];
                    let y1 = current_y + cmd.params[i + 1];
                    let x2 = current_x + cmd.params[i + 2];
                    let y2 = current_y + cmd.params[i + 3];
                    let x = current_x + cmd.params[i + 4];
                    let y = current_y + cmd.params[i + 5];

                    add_point(
                        [0.5 * (current_x + x1), 0.5 * (current_y + y1)],
                        &mut current_subpath_points,
                        &mut current_subpath_bbox,
                        &mut global_bbox,
                    );
                    add_point(
                        [0.5 * (x1 + x2), 0.5 * (y1 + y2)],
                        &mut current_subpath_points,
                        &mut current_subpath_bbox,
                        &mut global_bbox,
                    );
                    add_point(
                        [0.5 * (x2 + x), 0.5 * (y2 + y)],
                        &mut current_subpath_points,
                        &mut current_subpath_bbox,
                        &mut global_bbox,
                    );
                    add_point([x, y], &mut current_subpath_points, &mut current_subpath_bbox, &mut global_bbox);

                    current_x = x;
                    current_y = y;
                    prev_control_x = x2;
                    prev_control_y = y2;
                    i += 6;
                }
                last_cmd_was_curve = true;
                last_cmd_was_quad = false;
            },
            'S' => {
                let mut i = 0;
                while i + 3 < cmd.params.len() {
                    let x2 = cmd.params[i];
                    let y2 = cmd.params[i + 1];
                    let x = cmd.params[i + 2];
                    let y = cmd.params[i + 3];

                    let (x1, y1) = if last_cmd_was_curve {
                        reflect((current_x, current_y), (prev_control_x, prev_control_y))
                    } else {
                        (current_x, current_y)
                    };

                    // SVGO Logic for S:
                    // addPoint(midpoint(base, x1))
                    // addPoint(midpoint(x1, x2))
                    // addPoint(midpoint(x2, x))
                    // addPoint(x)

                    add_point(
                        [0.5 * (current_x + x1), 0.5 * (current_y + y1)],
                        &mut current_subpath_points,
                        &mut current_subpath_bbox,
                        &mut global_bbox,
                    );
                    add_point(
                        [0.5 * (x1 + x2), 0.5 * (y1 + y2)],
                        &mut current_subpath_points,
                        &mut current_subpath_bbox,
                        &mut global_bbox,
                    );
                    add_point(
                        [0.5 * (x2 + x), 0.5 * (y2 + y)],
                        &mut current_subpath_points,
                        &mut current_subpath_bbox,
                        &mut global_bbox,
                    );
                    add_point([x, y], &mut current_subpath_points, &mut current_subpath_bbox, &mut global_bbox);

                    current_x = x;
                    current_y = y;
                    prev_control_x = x2;
                    prev_control_y = y2;
                    i += 4;
                }
                last_cmd_was_curve = true;
                last_cmd_was_quad = false;
            },
            's' => {
                let mut i = 0;
                while i + 3 < cmd.params.len() {
                    let x2 = current_x + cmd.params[i];
                    let y2 = current_y + cmd.params[i + 1];
                    let x = current_x + cmd.params[i + 2];
                    let y = current_y + cmd.params[i + 3];

                    let (x1, y1) = if last_cmd_was_curve {
                        reflect((current_x, current_y), (prev_control_x, prev_control_y))
                    } else {
                        (current_x, current_y)
                    };

                    add_point(
                        [0.5 * (current_x + x1), 0.5 * (current_y + y1)],
                        &mut current_subpath_points,
                        &mut current_subpath_bbox,
                        &mut global_bbox,
                    );
                    add_point(
                        [0.5 * (x1 + x2), 0.5 * (y1 + y2)],
                        &mut current_subpath_points,
                        &mut current_subpath_bbox,
                        &mut global_bbox,
                    );
                    add_point(
                        [0.5 * (x2 + x), 0.5 * (y2 + y)],
                        &mut current_subpath_points,
                        &mut current_subpath_bbox,
                        &mut global_bbox,
                    );
                    add_point([x, y], &mut current_subpath_points, &mut current_subpath_bbox, &mut global_bbox);

                    current_x = x;
                    current_y = y;
                    prev_control_x = x2;
                    prev_control_y = y2;
                    i += 4;
                }
                last_cmd_was_curve = true;
                last_cmd_was_quad = false;
            },
            'Q' => {
                let mut i = 0;
                while i + 3 < cmd.params.len() {
                    let x1 = cmd.params[i];
                    let y1 = cmd.params[i + 1];
                    let x = cmd.params[i + 2];
                    let y = cmd.params[i + 3];

                    add_point([x1, y1], &mut current_subpath_points, &mut current_subpath_bbox, &mut global_bbox);
                    add_point([x, y], &mut current_subpath_points, &mut current_subpath_bbox, &mut global_bbox);

                    current_x = x;
                    current_y = y;
                    prev_control_x = x1;
                    prev_control_y = y1;
                    i += 4;
                }
                last_cmd_was_curve = false;
                last_cmd_was_quad = true;
            },
            'q' => {
                let mut i = 0;
                while i + 3 < cmd.params.len() {
                    let x1 = current_x + cmd.params[i];
                    let y1 = current_y + cmd.params[i + 1];
                    let x = current_x + cmd.params[i + 2];
                    let y = current_y + cmd.params[i + 3];

                    add_point([x1, y1], &mut current_subpath_points, &mut current_subpath_bbox, &mut global_bbox);
                    add_point([x, y], &mut current_subpath_points, &mut current_subpath_bbox, &mut global_bbox);

                    current_x = x;
                    current_y = y;
                    prev_control_x = x1;
                    prev_control_y = y1;
                    i += 4;
                }
                last_cmd_was_curve = false;
                last_cmd_was_quad = true;
            },
            'T' => {
                let mut i = 0;
                while i + 1 < cmd.params.len() {
                    let x = cmd.params[i];
                    let y = cmd.params[i + 1];

                    let (x1, y1) = if last_cmd_was_quad {
                        reflect((current_x, current_y), (prev_control_x, prev_control_y))
                    } else {
                        (current_x, current_y)
                    };

                    add_point([x1, y1], &mut current_subpath_points, &mut current_subpath_bbox, &mut global_bbox);
                    add_point([x, y], &mut current_subpath_points, &mut current_subpath_bbox, &mut global_bbox);

                    current_x = x;
                    current_y = y;
                    prev_control_x = x1;
                    prev_control_y = y1;
                    i += 2;
                }
                last_cmd_was_curve = false;
                last_cmd_was_quad = true;
            },
            't' => {
                let mut i = 0;
                while i + 1 < cmd.params.len() {
                    let x = current_x + cmd.params[i];
                    let y = current_y + cmd.params[i + 1];

                    let (x1, y1) = if last_cmd_was_quad {
                        reflect((current_x, current_y), (prev_control_x, prev_control_y))
                    } else {
                        (current_x, current_y)
                    };

                    add_point([x1, y1], &mut current_subpath_points, &mut current_subpath_bbox, &mut global_bbox);
                    add_point([x, y], &mut current_subpath_points, &mut current_subpath_bbox, &mut global_bbox);

                    current_x = x;
                    current_y = y;
                    prev_control_x = x1;
                    prev_control_y = y1;
                    i += 2;
                }
                last_cmd_was_curve = false;
                last_cmd_was_quad = true;
            },
            'A' => {
                let mut i = 0;
                while i + 6 < cmd.params.len() {
                    let rx = cmd.params[i];
                    let ry = cmd.params[i + 1];
                    let x_axis_rotation = cmd.params[i + 2];
                    let large_arc_flag = cmd.params[i + 3];
                    let sweep_flag = cmd.params[i + 4];
                    let x = cmd.params[i + 5];
                    let y = cmd.params[i + 6];

                    // Convert Arc to Cubic Beziers
                    let cubics =
                        arc_to_cubic(current_x, current_y, rx, ry, x_axis_rotation, large_arc_flag, sweep_flag, x, y);

                    let mut prev_x = current_x;
                    let mut prev_y = current_y;

                    for cubic in cubics {
                        let (cp1x, cp1y, cp2x, cp2y, endx, endy) = cubic;

                        // Add midpoints like 'C' command
                        add_point(
                            [0.5 * (prev_x + cp1x), 0.5 * (prev_y + cp1y)],
                            &mut current_subpath_points,
                            &mut current_subpath_bbox,
                            &mut global_bbox,
                        );
                        add_point(
                            [0.5 * (cp1x + cp2x), 0.5 * (cp1y + cp2y)],
                            &mut current_subpath_points,
                            &mut current_subpath_bbox,
                            &mut global_bbox,
                        );
                        add_point(
                            [0.5 * (cp2x + endx), 0.5 * (cp2y + endy)],
                            &mut current_subpath_points,
                            &mut current_subpath_bbox,
                            &mut global_bbox,
                        );
                        add_point(
                            [endx, endy],
                            &mut current_subpath_points,
                            &mut current_subpath_bbox,
                            &mut global_bbox,
                        );

                        prev_x = endx;
                        prev_y = endy;
                    }

                    current_x = x;
                    current_y = y;
                    i += 7;
                }
                last_cmd_was_curve = false;
                last_cmd_was_quad = false;
            },
            'a' => {
                let mut i = 0;
                while i + 6 < cmd.params.len() {
                    let rx = cmd.params[i];
                    let ry = cmd.params[i + 1];
                    let x_axis_rotation = cmd.params[i + 2];
                    let large_arc_flag = cmd.params[i + 3];
                    let sweep_flag = cmd.params[i + 4];
                    let x = current_x + cmd.params[i + 5];
                    let y = current_y + cmd.params[i + 6];

                    let cubics =
                        arc_to_cubic(current_x, current_y, rx, ry, x_axis_rotation, large_arc_flag, sweep_flag, x, y);

                    let mut prev_x = current_x;
                    let mut prev_y = current_y;

                    for cubic in cubics {
                        let (cp1x, cp1y, cp2x, cp2y, endx, endy) = cubic;

                        add_point(
                            [0.5 * (prev_x + cp1x), 0.5 * (prev_y + cp1y)],
                            &mut current_subpath_points,
                            &mut current_subpath_bbox,
                            &mut global_bbox,
                        );
                        add_point(
                            [0.5 * (cp1x + cp2x), 0.5 * (cp1y + cp2y)],
                            &mut current_subpath_points,
                            &mut current_subpath_bbox,
                            &mut global_bbox,
                        );
                        add_point(
                            [0.5 * (cp2x + endx), 0.5 * (cp2y + endy)],
                            &mut current_subpath_points,
                            &mut current_subpath_bbox,
                            &mut global_bbox,
                        );
                        add_point(
                            [endx, endy],
                            &mut current_subpath_points,
                            &mut current_subpath_bbox,
                            &mut global_bbox,
                        );

                        prev_x = endx;
                        prev_y = endy;
                    }

                    current_x = x;
                    current_y = y;
                    i += 7;
                }
                last_cmd_was_curve = false;
                last_cmd_was_quad = false;
            },
            'Z' | 'z' => {
                current_x = start_x;
                current_y = start_y;
                add_point(
                    [current_x, current_y],
                    &mut current_subpath_points,
                    &mut current_subpath_bbox,
                    &mut global_bbox,
                );
                last_cmd_was_curve = false;
                last_cmd_was_quad = false;
            },
            _ => {},
        }
    }

    flush_subpath(&mut current_subpath_points, &mut current_subpath_bbox, &mut subpaths);
    (subpaths, global_bbox)
}

// Convex Hull Algorithm (Monotone Chain)
fn convex_hull(points: &[Point]) -> Vec<Point> {
    if points.is_empty() {
        return Vec::new();
    }

    let mut points = points.to_vec();
    // Sort by x, then y
    points.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap().then(a[1].partial_cmp(&b[1]).unwrap()));

    // Build lower hull
    let mut lower: Vec<Point> = Vec::new();
    for p in &points {
        while lower.len() >= 2 {
            let p1 = lower[lower.len() - 2];
            let p2 = lower[lower.len() - 1];
            if cross(p1, p2, *p) <= 0.0 {
                lower.pop();
            } else {
                break;
            }
        }
        lower.push(*p);
    }

    // Build upper hull
    let mut upper: Vec<Point> = Vec::new();
    for p in points.iter().rev() {
        while upper.len() >= 2 {
            let p1 = upper[upper.len() - 2];
            let p2 = upper[upper.len() - 1];
            if cross(p1, p2, *p) <= 0.0 {
                upper.pop();
            } else {
                break;
            }
        }
        upper.push(*p);
    }

    // Concatenate (remove last point of each as it's repeated)
    if !upper.is_empty() {
        upper.pop();
    }
    if !lower.is_empty() {
        lower.pop();
    }

    let mut hull = lower;
    hull.extend(upper);
    hull
}

// GJK Algorithm
fn gjk_intersects(hull1: &[Point], hull2: &[Point]) -> bool {
    let mut direction = [1.0, 0.0]; // initial direction
    let mut simplex: Vec<Point> = Vec::with_capacity(3);

    // Initial point
    let s = get_support(hull1, hull2, direction);
    simplex.push(s);
    direction = minus(s);

    let mut iterations = 0;
    while iterations < 10000 {
        iterations += 1;

        let a = get_support(hull1, hull2, direction);
        simplex.push(a);

        if dot(direction, a) <= 0.0 {
            return false;
        }

        if process_simplex(&mut simplex, &mut direction) {
            return true;
        }
    }

    true // Safety fallback (infinite loop protection)
}

fn get_support(hull1: &[Point], hull2: &[Point], direction: Point) -> Point {
    let p1 = support_point(hull1, direction);
    let p2 = support_point(hull2, minus(direction));
    sub(p1, p2)
}

fn support_point(poly: &[Point], direction: Point) -> Point {
    let mut best_pt = poly[0];
    let mut max_dot = dot(best_pt, direction);

    for &p in poly.iter().skip(1) {
        let d = dot(p, direction);
        if d > max_dot {
            max_dot = d;
            best_pt = p;
        }
    }
    best_pt
}

fn process_simplex(simplex: &mut Vec<Point>, direction: &mut Point) -> bool {
    let len = simplex.len();
    let a = simplex[len - 1];

    if len == 2 {
        let b = simplex[0];
        let ao = minus(a);
        let ab = sub(b, a);

        if dot(ab, ao) > 0.0 {
            *direction = orth(ab, a);
        } else {
            *direction = ao;
            simplex.remove(0);
        }
        return false;
    } else if len == 3 {
        let b = simplex[1];
        let c = simplex[0];
        let ab = sub(b, a);
        let ac = sub(c, a);
        let ao = minus(a);

        let acb = orth(ab, ac);
        let abc = orth(ac, ab);

        if dot(acb, ao) > 0.0 {
            if dot(ab, ao) > 0.0 {
                *direction = acb;
                simplex.remove(0);
            } else {
                *direction = ao;
                simplex.remove(0);
                simplex.remove(0);
            }
        } else if dot(abc, ao) > 0.0 {
            if dot(ac, ao) > 0.0 {
                *direction = abc;
                simplex.remove(1);
            } else {
                *direction = ao;
                simplex.remove(0);
                simplex.remove(0);
            }
        } else {
            return true;
        }
        return false;
    }

    false
}

// Arc to Cubic Bezier conversion
// Ported from SVGO _path.js a2c function (which cites Snap.svg)
fn arc_to_cubic(
    x1: f64,
    y1: f64,
    rx: f64,
    ry: f64,
    angle: f64,
    large_arc_flag: f64,
    sweep_flag: f64,
    x2: f64,
    y2: f64,
) -> Vec<(f64, f64, f64, f64, f64, f64)> {
    let _120 = std::f64::consts::PI * 120.0 / 180.0;
    let rad = std::f64::consts::PI / 180.0 * angle;

    let mut res = Vec::new();

    let rotate_x = |x: f64, y: f64, rad: f64| x * rad.cos() - y * rad.sin();
    let rotate_y = |x: f64, y: f64, rad: f64| x * rad.sin() + y * rad.cos();

    let x1_rot = rotate_x(x1, y1, -rad);
    let y1_rot = rotate_y(x1, y1, -rad);
    let x2_rot = rotate_x(x2, y2, -rad);
    let y2_rot = rotate_y(x2, y2, -rad);

    let x = (x1_rot - x2_rot) / 2.0;
    let y = (y1_rot - y2_rot) / 2.0;

    let mut rx = rx.abs();
    let mut ry = ry.abs();

    let h = (x * x) / (rx * rx) + (y * y) / (ry * ry);
    if h > 1.0 {
        let h_sqrt = h.sqrt();
        rx *= h_sqrt;
        ry *= h_sqrt;
    }

    let rx2 = rx * rx;
    let ry2 = ry * ry;

    let k_numerator = (rx2 * ry2 - rx2 * y * y - ry2 * x * x).abs();
    let k_denominator = rx2 * y * y + ry2 * x * x;
    let k = if k_denominator == 0.0 {
        0.0
    } else {
        (k_numerator / k_denominator).sqrt()
    };

    let k = if large_arc_flag == sweep_flag { -k } else { k };

    let cx = k * rx * y / ry + (x1_rot + x2_rot) / 2.0;
    let cy = k * -ry * x / rx + (y1_rot + y2_rot) / 2.0;

    let f1 = ((y1_rot - cy) / ry).clamp(-1.0, 1.0).asin();
    let f2 = ((y2_rot - cy) / ry).clamp(-1.0, 1.0).asin();

    let mut f1 = if x1_rot < cx {
        std::f64::consts::PI - f1
    } else {
        f1
    };
    let mut f2 = if x2_rot < cx {
        std::f64::consts::PI - f2
    } else {
        f2
    };

    if f1 < 0.0 {
        f1 += std::f64::consts::PI * 2.0;
    }
    if f2 < 0.0 {
        f2 += std::f64::consts::PI * 2.0;
    }

    if sweep_flag != 0.0 && f1 > f2 {
        f1 -= std::f64::consts::PI * 2.0;
    }
    if sweep_flag == 0.0 && f2 > f1 {
        f2 -= std::f64::consts::PI * 2.0;
    }

    let mut current_f1 = f1;

    while (f2 - current_f1).abs() > _120 {
        let f2_split = current_f1
            + _120
                * (if sweep_flag != 0.0 && f2 > current_f1 {
                    1.0
                } else {
                    -1.0
                });
        let cubics = compute_cubic_segment(current_f1, f2_split, cx, cy, rx, ry, rad, rotate_x, rotate_y);
        res.push(cubics);
        current_f1 = f2_split;
    }

    let cubics = compute_cubic_segment(current_f1, f2, cx, cy, rx, ry, rad, rotate_x, rotate_y);
    res.push(cubics);

    res
}

fn compute_cubic_segment<F1, F2>(
    f1: f64,
    f2: f64,
    cx: f64,
    cy: f64,
    rx: f64,
    ry: f64,
    rad: f64,
    rotate_x: F1,
    rotate_y: F2,
) -> (f64, f64, f64, f64, f64, f64)
where
    F1: Fn(f64, f64, f64) -> f64,
    F2: Fn(f64, f64, f64) -> f64,
{
    let df = f2 - f1;
    let c1 = f1.cos();
    let s1 = f1.sin();
    let c2 = f2.cos();
    let s2 = f2.sin();

    let t = (df / 4.0).tan();
    let hx = 4.0 / 3.0 * rx * t;
    let hy = 4.0 / 3.0 * ry * t;

    let x1 = cx + rx * c1;
    let y1 = cy + ry * s1;
    let x2 = cx + rx * c2;
    let y2 = cy + ry * s2;

    let m1_x = x1 - hx * s1;
    let m1_y = y1 + hy * c1;
    let m2_x = x2 + hx * s2;
    let m2_y = y2 - hy * c2;

    (
        rotate_x(m1_x, m1_y, rad),
        rotate_y(m1_x, m1_y, rad),
        rotate_x(m2_x, m2_y, rad),
        rotate_y(m2_x, m2_y, rad),
        rotate_x(x2, y2, rad),
        rotate_y(x2, y2, rad),
    )
}

// Vector math helpers
fn dot(v1: Point, v2: Point) -> f64 {
    v1[0] * v2[0] + v1[1] * v2[1]
}

fn minus(v: Point) -> Point {
    [-v[0], -v[1]]
}

fn sub(v1: Point, v2: Point) -> Point {
    [v1[0] - v2[0], v1[1] - v2[1]]
}

fn cross(o: Point, a: Point, b: Point) -> f64 {
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
}

fn orth(v: Point, from: Point) -> Point {
    let o = [-v[1], v[0]];
    if dot(o, minus(from)) < 0.0 {
        minus(o)
    } else {
        o
    }
}

#[allow(dead_code)]
fn triple_product(a: Point, b: Point, c: Point) -> Point {
    let z = a[0] * b[1] - a[1] * b[0];
    [-z * c[1], z * c[0]]
}

static COMMAND_WS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*([MmZzLlHhVvCcSsQqTtAa])\s*").unwrap());
static MULTI_WS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

fn normalize_merged_path_data(path_data: &str) -> String {
    let mut normalized = path_data.replace(',', " ");
    normalized = COMMAND_WS_RE.replace_all(&normalized, "$1").to_string();
    normalized = MULTI_WS_RE.replace_all(&normalized, " ").to_string();
    normalized.trim().to_string()
}

/// Stringify path commands with SVGO-compatible implicit lineto merging.
/// Per SVG spec, coordinates after M/m are implicit L/l commands.
/// SVGO exploits this: `M x y L x2 y2` → `M x y x2 y2`.
fn stringify_path_data_svgo(commands: &[PathCommand]) -> String {
    use vexy_svgo_core::utils::numbers::NumberUtils;

    if commands.is_empty() {
        return String::new();
    }

    let mut merged: Vec<(char, Vec<f64>)> = Vec::with_capacity(commands.len());

    for cmd in commands {
        let can_merge = if let Some(last) = merged.last() {
            let (prev_cmd, _) = last;
            // Same command (except M/m which shouldn't be collapsed)
            (*prev_cmd == cmd.command && *prev_cmd != 'M' && *prev_cmd != 'm')
            // M followed by L → implicit lineto
            || (*prev_cmd == 'M' && cmd.command == 'L')
            // m followed by l → implicit relative lineto
            || (*prev_cmd == 'm' && cmd.command == 'l')
        } else {
            false
        };

        if can_merge {
            let last = merged.last_mut().unwrap();
            last.1.extend_from_slice(&cmd.params);
        } else {
            merged.push((cmd.command, cmd.params.clone()));
        }
    }

    // Check if leading M should absorb following L (SVGO lines 313-318)
    if merged.len() >= 2 {
        if merged[0].0 == 'M' && merged[1].0 == 'L' {
            let l_args = merged.remove(1).1;
            merged[0].1.extend(l_args);
        } else if merged[0].0 == 'm' && merged[1].0 == 'l' {
            let l_args = merged.remove(1).1;
            merged[0].1.extend(l_args);
        }
    }

    let mut result = String::new();
    for (cmd, params) in &merged {
        result.push(*cmd);
        for (i, &p) in params.iter().enumerate() {
            let mut formatted = NumberUtils::format_number_minimal(p);

            if 0.0 < p && p < 1.0 && formatted.starts_with('0') {
                formatted = formatted[1..].to_string();
            } else if -1.0 < p && p < 0.0 && formatted.len() > 2 && formatted.as_bytes()[1] == b'0' {
                formatted = format!("-{}", &formatted[2..]);
            }

            if i == 0 {
                result.push_str(&formatted);
            } else if p < 0.0 {
                result.push_str(&formatted);
            } else if !formatted.starts_with('.') {
                result.push(' ');
                result.push_str(&formatted);
            } else {
                result.push_str(&formatted);
            }
        }
    }

    result
}

fn normalize_subpath_start(path_data: &str) -> String {
    let trimmed = path_data.trim_start();
    if let Some(rest) = trimmed.strip_prefix('m') {
        // After 'm x,y', subsequent coordinate pairs are implicit relative 'l'.
        // When converting to 'M x,y', we must insert explicit 'l' to preserve
        // the relative lineto semantics for subsequent coordinates.
        let rest_trimmed = rest.trim_start();
        // Find end of first coordinate pair (first x,y after M)
        // Look for the second space/comma-separated coordinate transition
        let mut chars = rest_trimmed.char_indices();
        let mut coord_count = 0;
        let mut in_number = false;
        let mut split_pos = None;

        for (i, ch) in chars.by_ref() {
            if ch.is_ascii_digit() || ch == '.' || ch == '-' || ch == '+' {
                if !in_number {
                    coord_count += 1;
                    in_number = true;
                    if coord_count == 3 {
                        // Third number starts = first implicit lineto coordinate
                        split_pos = Some(i);
                        break;
                    }
                }
            } else if ch == ',' || ch == ' ' {
                in_number = false;
            } else if ch.is_ascii_alphabetic() {
                // Hit another command letter before 3rd number — no implicit lineto
                break;
            }
        }

        if let Some(pos) = split_pos {
            format!("M{}l{}", &rest_trimmed[..pos].trim_end(), &rest_trimmed[pos..])
        } else {
            format!("M{rest}")
        }
    } else {
        trimmed.to_string()
    }
}

impl MergePathsPlugin {
    /// Create a new MergePathsPlugin
    pub fn new() -> Self {
        Self {
            config: MergePathsConfig::default(),
        }
    }

    /// Create a new MergePathsPlugin with config
    pub fn with_config(config: MergePathsConfig) -> Self {
        Self { config }
    }

    /// Parse configuration from JSON
    #[allow(dead_code)]
    fn parse_config(params: &Value) -> Result<MergePathsConfig> {
        let mut config = MergePathsConfig::default();

        if let Some(obj) = params.as_object() {
            if let Some(force) = obj.get("force") {
                config.force = force.as_bool().unwrap_or(false);
            }
            if let Some(precision) = obj.get("floatPrecision") {
                config.float_precision = precision.as_u64().unwrap_or(3) as u8;
            }
            if let Some(no_space) = obj.get("noSpaceAfterFlags") {
                config.no_space_after_flags = no_space.as_bool().unwrap_or(false);
            }
        }

        Ok(config)
    }
}

impl Default for MergePathsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for MergePathsPlugin {
    fn name(&self) -> &'static str {
        "mergePaths"
    }

    fn description(&self) -> &'static str {
        "Merge compatible path elements"
    }

    fn validate_params(&self, params: &Value) -> anyhow::Result<()> {
        // Validate configuration parameters
        if let Some(obj) = params.as_object() {
            if let Some(force) = obj.get("force") {
                if !force.is_boolean() {
                    return Err(anyhow::anyhow!("force parameter must be a boolean"));
                }
            }
            if let Some(precision) = obj.get("floatPrecision") {
                if !precision.is_u64() {
                    return Err(anyhow::anyhow!("floatPrecision parameter must be a number"));
                }
            }
            if let Some(no_space) = obj.get("noSpaceAfterFlags") {
                if !no_space.is_boolean() {
                    return Err(anyhow::anyhow!("noSpaceAfterFlags parameter must be a boolean"));
                }
            }
        }
        Ok(())
    }

    fn configure(&mut self, params: &Value) -> anyhow::Result<()> {
        self.config = Self::parse_config(params)?;
        Ok(())
    }

    fn apply(&self, document: &mut Document) -> anyhow::Result<()> {
        let dangerous_classes = collect_dangerous_css_classes(&document.root);
        let mut visitor = PathMergeVisitor::new(self.config.clone(), dangerous_classes);
        vexy_svgo_core::visitor::walk_document(&mut visitor, document)?;
        Ok(())
    }
}

/// Collect CSS class names that have marker/clip-path/mask properties from `<style>` elements.
/// This is a lightweight alternative to full `computeStyle` — matches SVGO behavior for
/// blocking merges when CSS-computed styles include properties that depend on path endpoints.
fn collect_dangerous_css_classes(element: &Element) -> std::collections::HashSet<String> {
    let mut classes = std::collections::HashSet::new();
    collect_style_text(element, &mut |text| {
        static MARKER_RULE_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r"\.([a-zA-Z_][\w-]*)\s*\{[^}]*(marker-start|marker-mid|marker-end|clip-path|mask-image|mask)\s*:",
            )
            .unwrap()
        });
        static URL_RULE_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\.([a-zA-Z_][\w-]*)\s*\{[^}]*(fill|filter|stroke)\s*:.*url\(").unwrap());
        for cap in MARKER_RULE_RE.captures_iter(text) {
            if let Some(class_name) = cap.get(1) {
                classes.insert(class_name.as_str().to_string());
            }
        }
        for cap in URL_RULE_RE.captures_iter(text) {
            if let Some(class_name) = cap.get(1) {
                classes.insert(class_name.as_str().to_string());
            }
        }
    });
    classes
}

fn collect_style_text<F: FnMut(&str)>(element: &Element, f: &mut F) {
    if element.name == "style" {
        for child in &element.children {
            if let Node::Text(text) = child {
                f(text);
            }
            if let Node::CData(text) = child {
                f(text);
            }
        }
    }
    for child in &element.children {
        if let Node::Element(ref el) = child {
            collect_style_text(el, f);
        }
    }
}

struct PathMergeVisitor {
    #[allow(dead_code)]
    config:                MergePathsConfig,
    dangerous_css_classes: std::collections::HashSet<String>,
}

impl PathMergeVisitor {
    fn new(config: MergePathsConfig, dangerous_css_classes: std::collections::HashSet<String>) -> Self {
        Self {
            config,
            dangerous_css_classes,
        }
    }

    fn element_has_dangerous_computed_style(&self, el: &Element) -> bool {
        if let Some(style) = el.attributes.get("style") {
            static DANGEROUS_STYLE_RE: LazyLock<Regex> = LazyLock::new(|| {
                Regex::new(r"(?:marker-start|marker-mid|marker-end|clip-path|mask-image|mask)\s*:").unwrap()
            });
            if DANGEROUS_STYLE_RE.is_match(style) {
                return true;
            }
            static URL_STYLE_RE: LazyLock<Regex> =
                LazyLock::new(|| Regex::new(r"(?:fill|filter|stroke)\s*:.*url\(").unwrap());
            if URL_STYLE_RE.is_match(style) {
                return true;
            }
        }
        if let Some(class_attr) = el.attributes.get("class") {
            for class_name in class_attr.split_whitespace() {
                if self.dangerous_css_classes.contains(class_name) {
                    return true;
                }
            }
        }
        false
    }

    /// Process children of an element to merge consecutive compatible paths
    fn merge_paths_in_children(&self, element: &mut Element) {
        if element.children.len() <= 1 {
            return;
        }

        let mut to_remove: Vec<usize> = Vec::new();
        // Accumulated path data mirrors SVGO's prevPathData
        let mut anchor_idx: Option<usize> = None;
        let mut accumulated_d: Option<String> = None;

        for j in 0..element.children.len() {
            let is_path = matches!(
                element.children.get(j),
                Some(Node::Element(elem)) if elem.name == "path" && elem.attributes.contains_key("d")
            );

            if !is_path {
                // Flush accumulated merge into anchor
                if let (Some(ai), Some(merged)) = (anchor_idx.take(), accumulated_d.take()) {
                    if let Some(Node::Element(ref mut anchor)) = element.children.get_mut(ai) {
                        anchor.attributes.insert("d".into(), merged.into());
                    }
                }
                continue;
            }

            // SVGO: computeStyle check — block merge if CSS gives this path
            // marker-start/mid/end, clip-path, mask, or mask-image
            let has_dangerous_css = if let Some(Node::Element(ref el)) = element.children.get(j) {
                self.element_has_dangerous_computed_style(el)
            } else {
                false
            };

            if has_dangerous_css {
                if let (Some(ai), Some(merged)) = (anchor_idx.take(), accumulated_d.take()) {
                    if let Some(Node::Element(ref mut anchor)) = element.children.get_mut(ai) {
                        anchor.attributes.insert("d".into(), merged.into());
                    }
                }
                anchor_idx = Some(j);
                let d = if let Some(Node::Element(ref el)) = element.children.get(j) {
                    el.attributes.get("d").map(|v| v.to_string()).unwrap_or_default()
                } else {
                    String::new()
                };
                accumulated_d = Some(d);
                continue;
            }

            match anchor_idx {
                None => {
                    // Start a new merge group
                    anchor_idx = Some(j);
                    let d = if let Some(Node::Element(ref el)) = element.children.get(j) {
                        el.attributes.get("d").map(|v| v.to_string()).unwrap_or_default()
                    } else {
                        String::new()
                    };
                    accumulated_d = Some(d);
                },
                Some(ai) => {
                    let can_merge = {
                        let anchor = match element.children.get(ai) {
                            Some(Node::Element(ref el)) => el,
                            _ => continue,
                        };
                        let candidate = match element.children.get(j) {
                            Some(Node::Element(ref el)) => el,
                            _ => continue,
                        };
                        let acc = accumulated_d.as_deref().unwrap_or("");
                        self.can_merge_with_accumulated(anchor, candidate, acc)
                    };

                    if can_merge {
                        // Merge: append candidate's d to accumulated
                        let candidate_d = if let Some(Node::Element(ref el)) = element.children.get(j) {
                            el.attributes.get("d").map(|v| v.to_string()).unwrap_or_default()
                        } else {
                            String::new()
                        };
                        let acc = accumulated_d.as_deref().unwrap_or("");
                        accumulated_d = Some(self.merge_path_data(acc, &candidate_d));
                        to_remove.push(j);
                    } else {
                        // Flush previous group
                        if let Some(merged) = accumulated_d.take() {
                            if let Some(Node::Element(ref mut anchor)) = element.children.get_mut(ai) {
                                anchor.attributes.insert("d".into(), merged.into());
                            }
                        }
                        // Start new group with j
                        anchor_idx = Some(j);
                        let d = if let Some(Node::Element(ref el)) = element.children.get(j) {
                            el.attributes.get("d").map(|v| v.to_string()).unwrap_or_default()
                        } else {
                            String::new()
                        };
                        accumulated_d = Some(d);
                    }
                },
            }
        }

        // Flush final group
        if let (Some(ai), Some(merged)) = (anchor_idx, accumulated_d) {
            if let Some(Node::Element(ref mut anchor)) = element.children.get_mut(ai) {
                anchor.attributes.insert("d".into(), merged.into());
            }
        }

        // Remove merged children in reverse order
        for &idx in to_remove.iter().rev() {
            element.children.remove(idx);
        }
    }

    #[allow(dead_code)]
    fn can_merge_paths(&self, path1: &Element, path2: &Element) -> bool {
        if !self.attrs_compatible(path1, path2) {
            return false;
        }
        if self.config.force {
            return true;
        }
        if let (Some(d1_str), Some(d2_str)) = (path1.attributes.get("d"), path2.attributes.get("d")) {
            if paths_intersect(d1_str, d2_str) {
                return false;
            }
        }
        true
    }

    fn attrs_compatible(&self, path1: &Element, path2: &Element) -> bool {
        if path1.name != "path" || path2.name != "path" {
            return false;
        }
        if path1.attributes.len() != path2.attributes.len() {
            return false;
        }
        if !path1.children.is_empty() || !path2.children.is_empty() {
            return false;
        }
        if path1.attributes.get("transform") != path2.attributes.get("transform") {
            return false;
        }
        let style_attrs = [
            "fill",
            "stroke",
            "stroke-width",
            "stroke-linecap",
            "stroke-linejoin",
            "stroke-dasharray",
            "stroke-dashoffset",
            "opacity",
            "fill-opacity",
            "stroke-opacity",
            "fill-rule",
            "clip-path",
            "mask",
            "filter",
            "marker-start",
            "marker-mid",
            "marker-end",
            "class",
            "style",
        ];
        for attr in &style_attrs {
            if path1.attributes.get(*attr) != path2.attributes.get(*attr) {
                return false;
            }
        }
        for marker_attr in &["marker-start", "marker-mid", "marker-end"] {
            if path1.attributes.contains_key(*marker_attr) || path2.attributes.contains_key(*marker_attr) {
                return false;
            }
        }
        for attr in &["fill", "stroke", "clip-path", "mask", "filter"] {
            for el in [path1, path2] {
                if let Some(val) = el.attributes.get(*attr) {
                    if val.contains("url(") {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn can_merge_with_accumulated(&self, path1: &Element, path2: &Element, prev_d: &str) -> bool {
        if !self.attrs_compatible(path1, path2) {
            return false;
        }
        if self.config.force {
            return true;
        }
        if let Some(d2_str) = path2.attributes.get("d") {
            if !prev_d.is_empty() && paths_intersect(prev_d, d2_str) {
                return false;
            }
        }
        true
    }

    /// Merge two path elements' path data with normalization
    fn merge_path_data(&self, d1: &str, d2: &str) -> String {
        use vexy_svgo_core::utils::paths::PathUtils;

        if d1.is_empty() {
            let normalized = normalize_subpath_start(d2);
            let commands = PathUtils::parse_path_data(&normalized);
            return normalize_merged_path_data(&stringify_path_data_svgo(&commands));
        }
        if d2.is_empty() {
            let commands = PathUtils::parse_path_data(d1);
            return normalize_merged_path_data(&stringify_path_data_svgo(&commands));
        }

        let mut result = normalize_merged_path_data(d1);
        let next = normalize_subpath_start(d2);

        if !result.ends_with(' ') {
            result.push(' ');
        }
        result.push_str(&next);

        let commands = PathUtils::parse_path_data(&result);

        // Remove redundant pure-moveto commands: M x y followed by M x2 y2
        // means the first M is useless (no drawing between them).
        // Only strip M with exactly 2 params (pure move); M with 4+ params
        // has implicit lineto args and actually draws.
        let mut filtered: Vec<PathCommand> = Vec::with_capacity(commands.len());
        for cmd in commands {
            if (cmd.command == 'M' || cmd.command == 'm') && cmd.params.len() == 2 {
                if let Some(last) = filtered.last() {
                    if (last.command == 'M' || last.command == 'm') && last.params.len() == 2 {
                        filtered.pop();
                    }
                }
            }
            filtered.push(cmd);
        }

        normalize_merged_path_data(&stringify_path_data_svgo(&filtered))
    }
}

impl Visitor<'_> for PathMergeVisitor {
    fn visit_element_enter(&mut self, element: &mut Element<'_>) -> Result<(), VexyError> {
        // Process children to merge consecutive compatible paths
        self.merge_paths_in_children(element);
        Ok(())
    }
}

#[cfg(test)]
mod unit_tests {
    use std::borrow::Cow;

    use serde_json::json;
    use vexy_svgo_core::ast::{Document, Element, Node};

    use super::*;

    fn create_element(name: &'static str) -> Element<'static> {
        let mut element = Element::new(name);
        element.name = Cow::Borrowed(name);
        element
    }

    fn create_path_element(d: &str) -> Element<'static> {
        let mut element = create_element("path");
        element.set_attr("d", d);
        element
    }

    fn create_path_element_with_attrs(d: &str, attrs: &[(&str, &str)]) -> Element<'static> {
        let mut element = create_path_element(d);
        for (key, value) in attrs {
            element.set_attr(*key, *value);
        }
        element
    }

    #[test]
    fn test_parse_config() {
        let params = json!({
            "force": true,
            "floatPrecision": 5,
            "noSpaceAfterFlags": true
        });
        let config = MergePathsPlugin::parse_config(&params).unwrap();
        assert!(config.force);
        assert_eq!(config.float_precision, 5);
        assert!(config.no_space_after_flags);
    }

    #[test]
    fn test_merge_path_data() {
        let visitor = PathMergeVisitor::new(MergePathsConfig::default(), std::collections::HashSet::new());

        let d1 = "M0 0 L10 10";
        let d2 = "M20 20 L30 30";

        let merged = visitor.merge_path_data(d1, d2);
        assert_eq!(merged, "M0 0 10 10M20 20 30 30");

        assert_eq!(visitor.merge_path_data("", d2), "M20 20 30 30");
        assert_eq!(visitor.merge_path_data(d1, ""), "M0 0 10 10");

        let d3 = "m 20 20 30 30";
        let merged2 = visitor.merge_path_data(d1, d3);
        assert_eq!(merged2, "M0 0 10 10M20 20l30 30");
    }

    #[test]
    fn test_plugin_apply_basic_merge() {
        let mut plugin = MergePathsPlugin::new();
        // Force merge to ignore intersection check for simple test
        plugin.config.force = true;

        let mut doc = Document::new();
        let mut root = Element::new("svg");

        let path1 = create_path_element("M0 0L10 10");
        let path2 = create_path_element("M20 20L30 30");

        root.children.push(Node::Element(path1));
        root.children.push(Node::Element(path2));
        doc.root = root;

        plugin.apply(&mut doc).unwrap();

        assert_eq!(doc.root.children.len(), 1);
        if let Node::Element(ref el) = doc.root.children[0] {
            assert_eq!(el.name, "path");
            assert_eq!(el.attributes.get("d").unwrap(), "M0 0 10 10M20 20 30 30");
        } else {
            panic!("Expected path element");
        }
    }

    #[test]
    fn test_plugin_apply_no_merge_different_attrs() {
        let mut plugin = MergePathsPlugin::new();
        plugin.config.force = true;

        let mut doc = Document::new();
        let mut root = Element::new("svg");

        let path1 = create_path_element_with_attrs("M0 0L10 10", &[("fill", "red")]);
        let path2 = create_path_element_with_attrs("M20 20L30 30", &[("fill", "blue")]);

        root.children.push(Node::Element(path1));
        root.children.push(Node::Element(path2));
        doc.root = root;

        plugin.apply(&mut doc).unwrap();

        assert_eq!(doc.root.children.len(), 2);
    }

    #[test]
    fn test_plugin_apply_merge_with_same_attrs() {
        let mut plugin = MergePathsPlugin::new();
        plugin.config.force = true;

        let mut doc = Document::new();
        let mut root = Element::new("svg");

        let path1 = create_path_element_with_attrs("M0 0L10 10", &[("fill", "red")]);
        let path2 = create_path_element_with_attrs("M20 20L30 30", &[("fill", "red")]);

        root.children.push(Node::Element(path1));
        root.children.push(Node::Element(path2));
        doc.root = root;

        plugin.apply(&mut doc).unwrap();

        assert_eq!(doc.root.children.len(), 1);
        if let Node::Element(ref el) = doc.root.children[0] {
            assert_eq!(el.attributes.get("fill").unwrap(), "red");
        }
    }

    #[test]
    fn test_plugin_apply_mixed_elements() {
        let mut plugin = MergePathsPlugin::new();
        plugin.config.force = true;

        let mut doc = Document::new();
        let mut root = Element::new("svg");

        let path1 = create_path_element("M0 0L10 10");
        let rect = create_element("rect");
        let path2 = create_path_element("M20 20L30 30");

        root.children.push(Node::Element(path1));
        root.children.push(Node::Element(rect));
        root.children.push(Node::Element(path2));
        doc.root = root;

        plugin.apply(&mut doc).unwrap();

        // Should not merge across non-path element
        assert_eq!(doc.root.children.len(), 3);
    }

    #[test]
    fn test_plugin_apply_merge_three_paths() {
        let mut plugin = MergePathsPlugin::new();
        plugin.config.force = true;

        let mut doc = Document::new();
        let mut root = Element::new("svg");

        let path1 = create_path_element("M0 0L10 10");
        let path2 = create_path_element("M20 20L30 30");
        let path3 = create_path_element("M40 40L50 50");

        root.children.push(Node::Element(path1));
        root.children.push(Node::Element(path2));
        root.children.push(Node::Element(path3));
        doc.root = root;

        plugin.apply(&mut doc).unwrap();

        assert_eq!(doc.root.children.len(), 1);
        if let Node::Element(ref el) = doc.root.children[0] {
            assert_eq!(el.attributes.get("d").unwrap(), "M0 0 10 10M20 20 30 30M40 40 50 50");
        }
    }
}

// Use parameterized testing framework for SVGO fixture tests
crate::plugin_fixture_tests!(MergePathsPlugin, "mergePaths");
