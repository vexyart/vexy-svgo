// this_file: test/plugins/reuse_paths.rs

//! Tests for the reusePaths plugin
//! Auto-generated from SVGO test fixtures

use vexy_svgo::{optimize, OptimizeOptions, Config, PluginConfig};
use vexy_svgo::config::Js2SvgOptions;
use serde_json::json;

fn test_plugin(input: &str, expected: &str, params: Option<serde_json::Value>) {
    let config = Config {
        plugins: vec![PluginConfig {
            name: "reusePaths".to_string(),
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
        "\nPlugin: reusePaths\nInput:\n{}\nExpected:\n{}\nActual:\n{}\n", 
        input, expected, output);
}

#[test]
fn test_reuse_paths_01() {

    let input = r#"<svg xmlns=\"http://www.w3.org/2000/svg\">
    <path id=\"test0\" d=\"M 10,50 l 20,30 L 20,30\"/>
    <path transform=\"translate(10, 10)\"
          d=\"M 10,50 c 20,30 40,50 60,70 C 20,30 40,50 60,70\"/>
    <path transform=\"translate(20, 20)\"
          d=\"M 10,50 c 20,30 40,50 60,70 C 20,30 40,50 60,70\"/>
    <path d=\"M 10,50 c 20,30 40,50 60,70 C 20,30 40,50 60,70\"/>
    <path id=\"test1\" d=\"M 10,50 l 20,30 L 20,30\"/>
    <path d=\"M 10,50 a 20,60 45 0,1 40,70 A 20,60 45 0,1 40,70\"/>
    <path d=\"M 20,30 a 20,60 45 0,1 40,70 A 20,60 45 0,1 40,70\"/>
    <g>
      <path id=\"test2\" d=\"M 10,50 l 20,30 L 20,30\"/>
    </g>
    <path d=\"M 10,50 c 20,30 40,50 60,70 C 20,30 40,50 60,70\"/>
</svg>"#;

    let expected = r#"<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\">
    <defs>
        <path d=\"M 10,50 l 20,30 L 20,30\" id=\"test0\"/>
        <path d=\"M 10,50 c 20,30 40,50 60,70 C 20,30 40,50 60,70\" id=\"reuse-0\"/>
    </defs>
    <use xlink:href=\"#test0\"/>
    <use transform=\"translate(10, 10)\" xlink:href=\"#reuse-0\"/>
    <use transform=\"translate(20, 20)\" xlink:href=\"#reuse-0\"/>
    <use xlink:href=\"#reuse-0\"/>
    <use id=\"test1\" xlink:href=\"#test0\"/>
    <path d=\"M 10,50 a 20,60 45 0,1 40,70 A 20,60 45 0,1 40,70\"/>
    <path d=\"M 20,30 a 20,60 45 0,1 40,70 A 20,60 45 0,1 40,70\"/>
    <g>
        <use id=\"test2\" xlink:href=\"#test0\"/>
    </g>
    <use xlink:href=\"#reuse-0\"/>
</svg>"#;

    let params = None;

    test_plugin(input, expected, params);
}

#[test]
fn test_reuse_paths_02() {

    let input = r#"<svg xmlns=\"http://www.w3.org/2000/svg\">
    <path id=\"test0\" d=\"M 10,50 l 20,30 L 20,30\"/>
    <path id=\"test1\" stroke=\"red\" d=\"M 10,50 l 20,30 L 20,30\"/>
    <path id=\"test2\" stroke=\"blue\" d=\"M 10,50 l 20,30 L 20,30\"/>
    <path id=\"test3\" d=\"M 10,50 l 20,30 L 20,30\"/>
    <path id=\"test4\" stroke=\"blue\" d=\"M 10,50 l 20,30 L 20,30\"/>
    <path id=\"test1\" stroke=\"red\" fill=\"green\" d=\"M 10,50 l 20,30 L 20,30\"/>
</svg>"#;

    let expected = r#"<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\">
    <defs>
        <path d=\"M 10,50 l 20,30 L 20,30\" id=\"test0\"/>
        <path stroke=\"blue\" d=\"M 10,50 l 20,30 L 20,30\" id=\"test2\"/>
    </defs>
    <use xlink:href=\"#test0\"/>
    <path id=\"test1\" stroke=\"red\" d=\"M 10,50 l 20,30 L 20,30\"/>
    <use xlink:href=\"#test2\"/>
    <use id=\"test3\" xlink:href=\"#test0\"/>
    <use id=\"test4\" xlink:href=\"#test2\"/>
    <path id=\"test1\" stroke=\"red\" fill=\"green\" d=\"M 10,50 l 20,30 L 20,30\"/>
</svg>"#;

    let params = None;

    test_plugin(input, expected, params);
}

#[test]
fn test_reuse_paths_03() {

    let input = r#"<svg viewBox=\"0 0 200 200\" xmlns=\"http://www.w3.org/2000/svg\">
    <text>
        text element
    </text>
</svg>"#;

    let expected = r#"<svg viewBox=\"0 0 200 200\" xmlns=\"http://www.w3.org/2000/svg\">
    <text>
        text element
    </text>
</svg>"#;

    let params = None;

    test_plugin(input, expected, params);
}

#[test]
fn test_reuse_paths_04() {
    // Don PROTECTED_23_ t remove and reuse the ID of the duplicate path if it PROTECTED_21_ t remove and reuse the ID of the duplicate path if it's already being linked
    // in an href by another node.
    let input = r#"<svg xmlns=\"http://www.w3.org/2000/svg\"
  xmlns:xlink=\"http://www.w3.org/1999/xlink\" viewBox=\"-29.947 60.987 69.975 102.505\">
  <g transform=\"translate(-59 64)\">
    <g id=\"b\">
      <path id=\"a\" fill=\"#000\" d=\"M0 0v1h.5Z\" transform=\"rotate(18 3.157 -.5)\"/>
      <use xlink:href=\"#a\" width=\"1\" height=\"1\" transform=\"scale(-1 1)\"/>
    </g>
    <use xlink:href=\"#b\" width=\"1\" height=\"1\" transform=\"rotate(72)\"/>
    <use xlink:href=\"#b\" width=\"1\" height=\"1\" transform=\"rotate(-72)\"/>
    <use xlink:href=\"#b\" width=\"1\" height=\"1\" transform=\"rotate(144)\"/>
    <use xlink:href=\"#b\" width=\"1\" height=\"1\" transform=\"rotate(-144)\"/>
  </g>
  <path id=\"c\" fill=\"#000\" d=\"M0 0v1h.5Z\" transform=\"rotate(18 3.157 -.5)\"/>
  <use xlink:href=\"#c\" width=\"1\" height=\"1\" transform=\"scale(-1 1)\"/>
</svg>"#;

    let expected = r#"<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" viewBox=\"-29.947 60.987 69.975 102.505\">
    <defs>
        <path fill=\"#000\" d=\"M0 0v1h.5Z\" id=\"reuse-0\"/>
    </defs>
    <g transform=\"translate(-59 64)\">
        <g id=\"b\">
            <use id=\"a\" transform=\"rotate(18 3.157 -.5)\" xlink:href=\"#reuse-0\"/>
            <use xlink:href=\"#a\" width=\"1\" height=\"1\" transform=\"scale(-1 1)\"/>
        </g>
        <use xlink:href=\"#b\" width=\"1\" height=\"1\" transform=\"rotate(72)\"/>
        <use xlink:href=\"#b\" width=\"1\" height=\"1\" transform=\"rotate(-72)\"/>
        <use xlink:href=\"#b\" width=\"1\" height=\"1\" transform=\"rotate(144)\"/>
        <use xlink:href=\"#b\" width=\"1\" height=\"1\" transform=\"rotate(-144)\"/>
    </g>
    <use id=\"c\" transform=\"rotate(18 3.157 -.5)\" xlink:href=\"#reuse-0\"/>
    <use xlink:href=\"#c\" width=\"1\" height=\"1\" transform=\"scale(-1 1)\"/>
</svg>"#;

    let params = None;

    test_plugin(input, expected, params);
}
</svg>"#;

    let expected = r#"<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" version=\"1.0\" viewBox=\"0 0 400 360\">
    <defs>
        <clipPath id=\"c\">
            <use xlink:href=\"#a\" width=\"100%\" height=\"100%\" overflow=\"visible\"/>
        </clipPath>
        <path d=\"M51.94 428.2c14.5-32.39 36.88-59.5 64.38-81.96 13.76-11.23 65.04-24.09 73.86-16.58 9.45 8.06 13.45 26.18 5.53 38.45-1.23 1.9-37.38 26.83-39.1 28.32-2.19 1.9-38.65 17.58-43.76 19.51-14.02 5.28-29.47 10.43-44.31 12.71-3.19.5-14.98 3.85-16.6-.45z\" id=\"a\"/>
    </defs>
    <g transform=\"matrix(.491 0 0 .491 10.63 63.15)\">
        <use xlink:href=\"#a\" width=\"100%\" height=\"100%\" fill=\"#fff\" fill-rule=\"evenodd\" clip-rule=\"evenodd\" overflow=\"visible\"/>
        <path fill=\"none\" stroke=\"#c8cacc\" stroke-miterlimit=\"3.86\" stroke-width=\"66.34\" d=\"M48.33 412.36c14.5-32.39 36.89-59.5 64.39-81.96 13.75-11.23 65.03-24.09 73.85-16.58 9.45 8.06 13.45 26.18 5.53 38.45-1.22 1.9-37.38 26.83-39.09 28.32-2.2 1.9-38.65 17.58-43.77 19.51-14.01 5.28-29.47 10.44-44.3 12.71-3.2.5-14.99 3.85-16.61-.45z\" clip-path=\"url(#c)\"/>
    </g>
</svg>"#;

    let params = None;

    test_plugin(input, expected, params);
}

#[test]
fn test_reuse_paths_06() {
    // Don't remove and reuse the ID of the duplicate path if it's already being linked
    // in an href by another node.
    let input = r#"<svg xmlns=\"http://www.w3.org/2000/svg\"
  xmlns:xlink=\"http://www.w3.org/1999/xlink\" viewBox=\"-29.947 60.987 69.975 102.505\">
  <g transform=\"translate(-59 64)\">
    <g id=\"b\">
      <path id=\"a\" fill=\"#000\" d=\"M0 0v1h.5Z\" transform=\"rotate(18 3.157 -.5)\"/>
      <use xlink:href=\"#a\" width=\"1\" height=\"1\" transform=\"scale(-1 1)\"/>
    </g>
    <use xlink:href=\"#b\" width=\"1\" height=\"1\" transform=\"rotate(72)\"/>
    <use xlink:href=\"#b\" width=\"1\" height=\"1\" transform=\"rotate(-72)\"/>
    <use xlink:href=\"#b\" width=\"1\" height=\"1\" transform=\"rotate(144)\"/>
    <use xlink:href=\"#b\" width=\"1\" height=\"1\" transform=\"rotate(-144)\"/>
  </g>
  <path id=\"c\" fill=\"#000\" d=\"M0 0v1h.5Z\" transform=\"rotate(18 3.157 -.5)\"/>
  <use xlink:href=\"#c\" width=\"1\" height=\"1\" transform=\"scale(-1 1)\"/>
</svg>"#;

    let expected = r#"<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" viewBox=\"-29.947 60.987 69.975 102.505\">
    <defs>
        <path fill=\"#000\" d=\"M0 0v1h.5Z\" id=\"reuse-0\"/>
    </defs>
    <g transform=\"translate(-59 64)\">
        <g id=\"b\">
            <use id=\"a\" transform=\"rotate(18 3.157 -.5)\" xlink:href=\"#reuse-0\"/>
            <use xlink:href=\"#a\" width=\"1\" height=\"1\" transform=\"scale(-1 1)\"/>
        </g>
        <use xlink:href=\"#b\" width=\"1\" height=\"1\" transform=\"rotate(72)\"/>
        <use xlink:href=\"#b\" width=\"1\" height=\"1\" transform=\"rotate(-72)\"/>
        <use xlink:href=\"#b\" width=\"1\" height=\"1\" transform=\"rotate(144)\"/>
        <use xlink:href=\"#b\" width=\"1\" height=\"1\" transform=\"rotate(-144)\"/>
    </g>
    <use id=\"c\" transform=\"rotate(18 3.157 -.5)\" xlink:href=\"#reuse-0\"/>
    <use xlink:href=\"#c\" width=\"1\" height=\"1\" transform=\"scale(-1 1)\"/>
</svg>"#;

    let params = None;

    test_plugin(input, expected, params);
}
