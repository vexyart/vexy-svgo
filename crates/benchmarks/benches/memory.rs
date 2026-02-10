// this_file: crates/benchmarks/benches/memory.rs

//! Memory usage benchmarks for AST allocation strategies

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use vexy_svgo_core::Parser;

const SIMPLE_SVG: &str = include_str!("../../../testdata/integration/simple.svg");
const COMPLEX_SVG: &str = include_str!("../../../testdata/integration/complex.svg");
const GLASS_SVG: &str = include_str!("../../../testdata/integration/glass.svg");

fn count_allocations_parse(svg: &str) -> (usize, usize) {
    let parser = Parser::new();
    let doc = parser.parse(svg).expect("parse failed");

    let nodes = count_nodes(&doc);
    let mem = doc.root.estimated_memory_usage();
    (nodes, mem)
}

fn count_nodes(doc: &vexy_svgo_core::ast::Document) -> usize {
    fn count_element(elem: &vexy_svgo_core::ast::Element) -> usize {
        1 + elem
            .children
            .iter()
            .map(|child| match child {
                vexy_svgo_core::ast::Node::Element(e) => count_element(e),
                _ => 1,
            })
            .sum::<usize>()
    }
    count_element(&doc.root)
}

fn bench_memory_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_simple");

    group.bench_function("parse_and_measure", |b| {
        b.iter(|| {
            let (nodes, mem) = count_allocations_parse(black_box(SIMPLE_SVG));
            black_box((nodes, mem));
        });
    });

    group.finish();
}

fn bench_memory_complex(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_complex");

    group.bench_function("parse_and_measure", |b| {
        b.iter(|| {
            let (nodes, mem) = count_allocations_parse(black_box(COMPLEX_SVG));
            black_box((nodes, mem));
        });
    });

    group.finish();
}

fn bench_baseline_current_allocator(c: &mut Criterion) {
    c.bench_function("baseline_current_allocator", |b| {
        let parser = Parser::new();
        b.iter(|| {
            let doc = parser.parse(black_box(GLASS_SVG)).expect("parse failed");
            let mem = doc.root.estimated_memory_usage();
            black_box(mem);
        });
    });
}

criterion_group!(benches, bench_memory_simple, bench_memory_complex, bench_baseline_current_allocator,);
criterion_main!(benches);
