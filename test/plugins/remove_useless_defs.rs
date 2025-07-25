// this_file: test/plugins/remove_useless_defs.rs

//! Tests for the removeUselessDefs plugin
//! Auto-generated from SVGO test fixtures

use vexy_svgo::{optimize, OptimizeOptions, Config, PluginConfig};
use vexy_svgo::config::Js2SvgOptions;
use serde_json::json;

fn test_plugin(input: &str, expected: &str, params: Option<serde_json::Value>) {
    let config = Config {
        plugins: vec![PluginConfig {
            name: "removeUselessDefs".to_string(),
            params,
        }],
        multipass: false,
        js2svg: Js2SvgOptions {
            pretty: true,
            indent: 4,
            ..Default::default()
        },
        ..Default::default()
    };

    let options = OptimizeOptions::new(config);
    let result = optimize(input, options).expect("Optimization should succeed");
    let output = result.data.trim();
    let expected = expected.trim();
    
    assert_eq!(output, expected, 
        "\nPlugin: removeUselessDefs\nInput:\n{}\nExpected:\n{}\nActual:\n{}\n", 
        input, expected, output);
}

#[test]
fn test_remove_useless_defs_01() {

    let input = r#"<svg>
    <defs>
        <path d=\"...\"/>
        <g>
            <path d=\"...\" id=\"a\"/>
        </g>
    </defs>
</svg>"#;

    let expected = r#"<svg>
    <defs>
        <path d=\"...\" id=\"a\"/>
    </defs>
</svg>"#;

    let params = None;

    test_plugin(input, expected, params);
}

#[test]
fn test_remove_useless_defs_02() {

    let input = r#"<svg>
    <linearGradient id=\"linear\">
        <stop/>
        <stop/>
    </linearGradient>
    <radialGradient id=\"radial\">
        <stop/>
        <stop/>
    </radialGradient>
    <pattern id=\"pattern\">
        <rect/>
    </pattern>
    <clipPath id=\"clip\">
        <path/>
    </clipPath>
    <mask id=\"mask\">
        <rect/>
    </mask>
    <marker id=\"marker\">
        <path/>
    </marker>
    <symbol id=\"symbol\">
        <rect/>
    </symbol>
    <solidColor id=\"color\"/>
</svg>"#;

    let expected = r#"<svg>
    <linearGradient id=\"linear\">
        <stop/>
        <stop/>
    </linearGradient>
    <radialGradient id=\"radial\">
        <stop/>
        <stop/>
    </radialGradient>
    <pattern id=\"pattern\">
        <rect/>
    </pattern>
    <clipPath id=\"clip\">
        <path/>
    </clipPath>
    <mask id=\"mask\">
        <rect/>
    </mask>
    <marker id=\"marker\">
        <path/>
    </marker>
    <symbol id=\"symbol\">
        <rect/>
    </symbol>
    <solidColor id=\"color\"/>
</svg>"#;

    let params = None;

    test_plugin(input, expected, params);
}

#[test]
fn test_remove_useless_defs_03() {

    let input = r#"<svg>
    <linearGradient>
        <stop/>
        <stop/>
    </linearGradient>
    <radialGradient>
        <stop/>
        <stop/>
    </radialGradient>
    <pattern>
        <rect/>
    </pattern>
    <clipPath>
        <path/>
    </clipPath>
    <mask>
        <rect/>
    </mask>
    <marker>
        <path/>
    </marker>
    <symbol>
        <rect/>
    </symbol>
    <solidColor/>
    <path/>
</svg>"#;

    let expected = r#"<svg>
    <path/>
</svg>"#;

    let params = None;

    test_plugin(input, expected, params);
}

#[test]
fn test_remove_useless_defs_04() {
    // Don PROTECTED_13_ t remove nodes that have children with referenced IDs.
    let input = r#"<svg xmlns=\"http://www.w3.org/2000/svg\">
    <rect fill=\"url(#a)\" width=\"64\" height=\"64\"/>
    <g>
        <linearGradient id=\"a\">
            <stop offset=\"5%\" stop-color=\"gold\" />
        </linearGradient>
    </g>
</svg>"#;

    let expected = r#"<svg xmlns=\"http://www.w3.org/2000/svg\">
    <rect fill=\"url(#a)\" width=\"64\" height=\"64\"/>
    <g>
        <linearGradient id=\"a\">
            <stop offset=\"5%\" stop-color=\"gold\"/>
        </linearGradient>
    </g>
</svg>"#;

    let params = None;

    test_plugin(input, expected, params);
}

#[test]
fn test_remove_useless_defs_05() {
    // Don't remove nodes that have children with referenced IDs.
    let input = r#"<svg xmlns=\"http://www.w3.org/2000/svg\">
    <rect fill=\"url(#a)\" width=\"64\" height=\"64\"/>
    <g>
        <linearGradient id=\"a\">
            <stop offset=\"5%\" stop-color=\"gold\" />
        </linearGradient>
    </g>
</svg>"#;

    let expected = r#"<svg xmlns=\"http://www.w3.org/2000/svg\">
    <rect fill=\"url(#a)\" width=\"64\" height=\"64\"/>
    <g>
        <linearGradient id=\"a\">
            <stop offset=\"5%\" stop-color=\"gold\"/>
        </linearGradient>
    </g>
</svg>"#;

    let params = None;

    test_plugin(input, expected, params);
}
