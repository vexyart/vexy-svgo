// this_file: crates/benchmarks/benches/optimization.rs

//! Performance benchmarks for vexy-svgo optimization

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use vexy_svgo_core::{optimize_default, optimize_with_config, Config};

const SIMPLE_SVG: &str = r#"<svg width="100" height="100">
    <rect x="10" y="10" width="80" height="80" fill="red"/>
</svg>"#;

const MEDIUM_SVG: &str = r#"<svg width="500" height="500" viewBox="0 0 500 500">
    <defs>
        <linearGradient id="grad1" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" style="stop-color:rgb(255,255,0);stop-opacity:1" />
            <stop offset="100%" style="stop-color:rgb(255,0,0);stop-opacity:1" />
        </linearGradient>
    </defs>
    <g transform="translate(50,50)">
        <rect width="100" height="100" fill="url(#grad1)" />
        <circle cx="200" cy="200" r="50" fill="blue" opacity="0.5"/>
        <path d="M 10 10 L 90 10 L 90 90 L 10 90 Z" fill="green"/>
        <text x="250" y="250" font-family="Arial" font-size="20">Hello World</text>
    </g>
</svg>"#;

const COMPLEX_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="800" height="600" viewBox="0 0 800 600">
    <defs>
        <pattern id="pattern1" x="0" y="0" width="20" height="20" patternUnits="userSpaceOnUse">
            <rect width="10" height="10" fill="#f0f0f0"/>
            <rect x="10" y="10" width="10" height="10" fill="#f0f0f0"/>
        </pattern>
        <filter id="blur">
            <feGaussianBlur in="SourceGraphic" stdDeviation="5"/>
        </filter>
        <mask id="mask1">
            <rect width="100%" height="100%" fill="white"/>
            <circle cx="400" cy="300" r="100" fill="black"/>
        </mask>
    </defs>
    <rect width="100%" height="100%" fill="url(#pattern1)"/>
    <g mask="url(#mask1)">
        <rect x="100" y="100" width="600" height="400" fill="blue" filter="url(#blur)"/>
        <g transform="rotate(45 400 300)">
            <rect x="350" y="250" width="100" height="100" fill="red" opacity="0.7"/>
            <ellipse cx="400" cy="300" rx="150" ry="100" fill="none" stroke="green" stroke-width="5"/>
        </g>
    </g>
    <text x="400" y="550" text-anchor="middle" font-family="Arial" font-size="30" fill="black">
        Complex SVG Example
    </text>
</svg>"##;

fn bench_simple_svg(c: &mut Criterion) {
    c.bench_function("optimize_simple_svg", |b| {
        b.iter(|| {
            let result = optimize_default(black_box(SIMPLE_SVG));
            assert!(result.is_ok());
        })
    });
}

fn bench_medium_svg(c: &mut Criterion) {
    c.bench_function("optimize_medium_svg", |b| {
        b.iter(|| {
            let result = optimize_default(black_box(MEDIUM_SVG));
            assert!(result.is_ok());
        })
    });
}

fn bench_complex_svg(c: &mut Criterion) {
    c.bench_function("optimize_complex_svg", |b| {
        b.iter(|| {
            let result = optimize_default(black_box(COMPLEX_SVG));
            assert!(result.is_ok());
        })
    });
}

fn bench_multipass(c: &mut Criterion) {
    let mut group = c.benchmark_group("multipass");

    for passes in [false, true] {
        group.bench_with_input(
            BenchmarkId::from_parameter(if passes { "enabled" } else { "disabled" }),
            &passes,
            |b, &passes| {
                let mut config = Config::default();
                config.multipass = passes;

                b.iter(|| {
                    let result = optimize_with_config(black_box(MEDIUM_SVG), config.clone());
                    assert!(result.is_ok());
                })
            },
        );
    }
    group.finish();
}

fn bench_pretty_print(c: &mut Criterion) {
    let mut group = c.benchmark_group("pretty_print");

    for pretty in [false, true] {
        group.bench_with_input(
            BenchmarkId::from_parameter(if pretty { "enabled" } else { "disabled" }),
            &pretty,
            |b, &pretty| {
                let mut config = Config::default();
                config.pretty = pretty;

                b.iter(|| {
                    let result = optimize_with_config(black_box(MEDIUM_SVG), config.clone());
                    assert!(result.is_ok());
                })
            },
        );
    }
    group.finish();
}

fn bench_large_svg(c: &mut Criterion) {
    // Generate a large SVG with many elements
    let mut large_svg = String::from(r#"<svg width="1000" height="1000">"#);
    for i in 0..1000 {
        large_svg.push_str(&format!(
            r##"<rect x="{}" y="{}" width="10" height="10" fill="#{:06x}"/>"##,
            (i % 100) * 10,
            (i / 100) * 10,
            i * 1000
        ));
    }
    large_svg.push_str("</svg>");

    c.bench_function("optimize_1000_elements", |b| {
        b.iter(|| {
            let result = optimize_default(black_box(&large_svg));
            assert!(result.is_ok());
        })
    });
}

fn bench_path_optimization(c: &mut Criterion) {
    let path_heavy_svg = r#"<svg>
        <path d="M 10 10 L 20 20 L 30 10 L 40 20 L 50 10 L 60 20 L 70 10 L 80 20 L 90 10"/>
        <path d="M 10 30 Q 20 40 30 30 T 50 30 T 70 30 T 90 30"/>
        <path d="M 10 50 C 20 60 30 60 40 50 S 60 40 70 50 S 90 60 100 50"/>
        <path d="M 10.123456 70.987654 L 20.234567 80.876543 L 30.345678 70.765432"/>
        <path d="M 0 0 h 100 v 100 h -100 z"/>
    </svg>"#;

    c.bench_function("optimize_path_heavy_svg", |b| {
        b.iter(|| {
            let result = optimize_default(black_box(path_heavy_svg));
            assert!(result.is_ok());
        })
    });
}

#[cfg(feature = "parallel")]
fn bench_parallel_processing(c: &mut Criterion) {
    use vexy_svgo_core::{optimize, OptimizeOptions};

    // Generate a very large SVG
    let mut huge_svg = String::from(r#"<svg width="2000" height="2000">"#);
    for i in 0..5000 {
        huge_svg.push_str(&format!(
            r##"<g transform="translate({}, {})">
                <rect width="20" height="20" fill="#ff0000"/>
                <circle cx="10" cy="10" r="5" fill="#00ff00"/>
                <path d="M 0 0 L 20 20" stroke="#0000ff"/>
            </g>"##,
            (i % 100) * 20,
            (i / 100) * 20
        ));
    }
    huge_svg.push_str("</svg>");

    let mut group = c.benchmark_group("parallel_processing");
    group.sample_size(10); // Reduce sample size for large benchmarks

    // Sequential processing
    group.bench_function("sequential", |b| {
        let config = Config::default();

        b.iter(|| {
            let options = OptimizeOptions::new(config.clone()).with_parallel(false);
            let result = optimize(black_box(&huge_svg), options);
            assert!(result.is_ok());
        })
    });

    // Parallel processing
    group.bench_function("parallel", |b| {
        let mut config = Config::default();
        config.parallel = Some(4);

        b.iter(|| {
            let options = OptimizeOptions::new(config.clone()).with_parallel(true);
            let result = optimize(black_box(&huge_svg), options);
            assert!(result.is_ok());
        })
    });

    group.finish();
}

fn bench_different_svg_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("svg_sizes");

    for size in [10, 100, 500, 1000] {
        let mut svg = String::from(r#"<svg>"#);
        for i in 0..size {
            svg.push_str(&format!(r#"<rect x="{}" y="0" width="1" height="1"/>"#, i));
        }
        svg.push_str("</svg>");

        group.bench_with_input(BenchmarkId::from_parameter(format!("{}_elements", size)), &svg, |b, svg| {
            b.iter(|| {
                let result = optimize_default(black_box(svg));
                assert!(result.is_ok());
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_simple_svg,
    bench_medium_svg,
    bench_complex_svg,
    bench_multipass,
    bench_pretty_print,
    bench_large_svg,
    bench_path_optimization,
    bench_different_svg_sizes,
);

#[cfg(feature = "parallel")]
criterion_group!(parallel_benches, bench_parallel_processing,);

#[cfg(not(feature = "parallel"))]
criterion_main!(benches);

#[cfg(feature = "parallel")]
criterion_main!(benches, parallel_benches);
