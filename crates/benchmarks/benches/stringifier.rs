//! Performance benchmarks for the stringifier

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use vexy_svgo_core::{stringify, Document, Element, Node, Parser};

fn create_simple_document() -> Document<'static> {
    let mut root = Element::new("svg");
    root.set_attr("width", "100");
    root.set_attr("height", "100");
    
    let mut rect = Element::new("rect");
    rect.set_attr("x", "10");
    rect.set_attr("y", "10");
    rect.set_attr("width", "80");
    rect.set_attr("height", "80");
    rect.set_attr("fill", "red");
    
    root.add_child(Node::Element(rect));
    
    Document {
        root,
        ..Default::default()
    }
}

fn create_nested_document(depth: usize) -> Document<'static> {
    let mut root = Element::new("svg");
    let mut current = &mut root;
    
    for _ in 0..depth {
        let mut group = Element::new("g");
        let rect = Element::new("rect");
        group.add_child(Node::Element(rect));
        current.add_child(Node::Element(group));
        
        if let Node::Element(ref mut elem) = current.children.last_mut().unwrap() {
            current = elem;
        }
    }
    
    Document {
        root,
        ..Default::default()
    }
}

fn create_document_with_many_attrs(attr_count: usize) -> Document<'static> {
    let mut root = Element::new("svg");
    
    let mut elem = Element::new("rect");
    for i in 0..attr_count {
        elem.set_attr(&format!("data-attr-{}", i), &format!("value-{}", i));
    }
    
    root.add_child(Node::Element(elem));
    
    Document {
        root,
        ..Default::default()
    }
}

fn bench_stringify_simple(c: &mut Criterion) {
    let doc = create_simple_document();
    
    c.bench_function("stringify_simple", |b| {
        b.iter(|| {
            let result = stringify(black_box(&doc));
            assert!(result.is_ok());
        })
    });
}

fn bench_stringify_nested(c: &mut Criterion) {
    let mut group = c.benchmark_group("stringify_nested");
    
    for depth in [5, 10, 20, 50] {
        let doc = create_nested_document(depth);
        
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("depth_{}", depth)),
            &doc,
            |b, doc| {
                b.iter(|| {
                    let result = stringify(black_box(doc));
                    assert!(result.is_ok());
                })
            },
        );
    }
    
    group.finish();
}

fn bench_stringify_many_attrs(c: &mut Criterion) {
    let mut group = c.benchmark_group("stringify_attributes");
    
    for count in [10, 50, 100, 500] {
        let doc = create_document_with_many_attrs(count);
        
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_attrs", count)),
            &doc,
            |b, doc| {
                b.iter(|| {
                    let result = stringify(black_box(doc));
                    assert!(result.is_ok());
                })
            },
        );
    }
    
    group.finish();
}

fn bench_stringify_text_content(c: &mut Criterion) {
    let mut root = Element::new("svg");
    
    // Add text elements with varying content
    for i in 0..100 {
        let mut text = Element::new("text");
        text.set_attr("x", &(i * 10).to_string());
        text.set_attr("y", &(i * 10).to_string());
        text.add_child(Node::Text(format!("Text content number {} with some additional text", i)));
        root.add_child(Node::Element(text));
    }
    
    let doc = Document {
        root,
        ..Default::default()
    };
    
    c.bench_function("stringify_text_heavy", |b| {
        b.iter(|| {
            let result = stringify(black_box(&doc));
            assert!(result.is_ok());
        })
    });
}

fn bench_stringify_mixed_content(c: &mut Criterion) {
    let mut root = Element::new("svg");
    
    // Add various types of content
    root.add_child(Node::Comment("This is a comment".to_string().into_boxed_str()));
    
    let mut group = Element::new("g");
    group.add_child(Node::Text("Some text".to_string()));
    
    let mut rect = Element::new("rect");
    rect.set_attr("width", "100");
    rect.set_attr("height", "100");
    group.add_child(Node::Element(rect));
    
    group.add_child(Node::CData("function() { return 'CDATA content'; }".to_string()));
    
    root.add_child(Node::Element(group));
    
    let doc = Document {
        root,
        ..Default::default()
    };
    
    c.bench_function("stringify_mixed_content", |b| {
        b.iter(|| {
            let result = stringify(black_box(&doc));
            assert!(result.is_ok());
        })
    });
}

fn bench_stringify_namespaces(c: &mut Criterion) {
    let mut root = Element::new("svg");
    root.namespaces.insert("".to_string(), "http://www.w3.org/2000/svg".to_string());
    root.namespaces.insert("xlink".to_string(), "http://www.w3.org/1999/xlink".to_string());
    root.namespaces.insert("custom".to_string(), "http://example.com/custom".to_string());
    
    for i in 0..50 {
        let mut elem = Element::new("use");
        elem.set_attr("xlink:href", &format!("#element{}", i));
        elem.set_attr("custom:data", &format!("value{}", i));
        root.add_child(Node::Element(elem));
    }
    
    let doc = Document {
        root,
        ..Default::default()
    };
    
    c.bench_function("stringify_with_namespaces", |b| {
        b.iter(|| {
            let result = stringify(black_box(&doc));
            assert!(result.is_ok());
        })
    });
}

fn bench_stringify_large_document(c: &mut Criterion) {
    let mut root = Element::new("svg");
    root.set_attr("width", "10000");
    root.set_attr("height", "10000");
    
    // Create a large document with many elements
    for i in 0..1000 {
        let mut group = Element::new("g");
        group.set_attr("transform", &format!("translate({}, {})", i * 10, i * 10));
        
        let mut rect = Element::new("rect");
        rect.set_attr("width", "10");
        rect.set_attr("height", "10");
        rect.set_attr("fill", &format!("#{:06x}", i * 1000));
        
        let mut circle = Element::new("circle");
        circle.set_attr("cx", "5");
        circle.set_attr("cy", "5");
        circle.set_attr("r", "3");
        
        group.add_child(Node::Element(rect));
        group.add_child(Node::Element(circle));
        root.add_child(Node::Element(group));
    }
    
    let doc = Document {
        root,
        ..Default::default()
    };
    
    c.bench_function("stringify_1000_groups", |b| {
        b.iter(|| {
            let result = stringify(black_box(&doc));
            assert!(result.is_ok());
        })
    });
}

fn bench_roundtrip(c: &mut Criterion) {
    let complex_svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="800" height="600">
        <defs>
            <linearGradient id="grad1">
                <stop offset="0%" style="stop-color:rgb(255,255,0);stop-opacity:1"/>
                <stop offset="100%" style="stop-color:rgb(255,0,0);stop-opacity:1"/>
            </linearGradient>
        </defs>
        <g transform="translate(100,100)">
            <rect width="200" height="200" fill="url(#grad1)"/>
            <text x="100" y="100" text-anchor="middle">Hello World</text>
        </g>
    </svg>"#;
    
    c.bench_function("parse_stringify_roundtrip", |b| {
        let parser = Parser::new();
        b.iter(|| {
            let doc = parser.parse(black_box(complex_svg)).unwrap();
            let result = stringify(&doc);
            assert!(result.is_ok());
        })
    });
}

criterion_group!(
    benches,
    bench_stringify_simple,
    bench_stringify_nested,
    bench_stringify_many_attrs,
    bench_stringify_text_content,
    bench_stringify_mixed_content,
    bench_stringify_namespaces,
    bench_stringify_large_document,
    bench_roundtrip,
);

criterion_main!(benches);