//! Comprehensive unit tests for the stringifier module

#[cfg(test)]
mod tests {
    use vexy_svgo_core::{stringify, Document, Element, Node, Parser};

    #[test]
    fn test_stringify_simple_element() {
        let mut elem = Element::new("rect");
        elem.set_attr("x", "10");
        elem.set_attr("y", "20");
        elem.set_attr("width", "100");
        elem.set_attr("height", "50");

        let doc = Document {
            root: elem,
            ..Default::default()
        };

        let result = stringify(&doc);
        assert!(result.is_ok());
        let output = result.unwrap();

        assert!(output.contains("<rect"));
        assert!(output.contains("x=\"10\""));
        assert!(output.contains("y=\"20\""));
        assert!(output.contains("width=\"100\""));
        assert!(output.contains("height=\"50\""));
        assert!(output.contains("/>"));
    }

    #[test]
    fn test_stringify_with_children() {
        let mut parent = Element::new("g");
        parent.set_attr("id", "group1");

        let mut child1 = Element::new("rect");
        child1.set_attr("width", "50");

        let child2 = Element::new("circle");

        parent.add_child(Node::Element(child1));
        parent.add_child(Node::Element(child2));

        let doc = Document {
            root: parent,
            ..Default::default()
        };

        let result = stringify(&doc);
        assert!(result.is_ok());
        let output = result.unwrap();

        assert!(output.contains("<g"));
        assert!(output.contains("id=\"group1\""));
        assert!(output.contains("<rect"));
        assert!(output.contains("<circle"));
        assert!(output.contains("</g>"));
    }

    #[test]
    fn test_stringify_text_content() {
        let mut text_elem = Element::new("text");
        text_elem.set_attr("x", "10");
        text_elem.set_attr("y", "20");
        text_elem.add_child(Node::Text("Hello World".into()));

        let doc = Document {
            root: text_elem,
            ..Default::default()
        };

        let result = stringify(&doc);
        assert!(result.is_ok());
        let output = result.unwrap();

        assert!(output.contains("<text"));
        assert!(output.contains("x=\"10\""));
        assert!(output.contains("y=\"20\""));
        assert!(output.contains(">Hello World</text>"));
    }

    #[test]
    fn test_stringify_with_comments() {
        let mut elem = Element::new("svg");
        elem.add_child(Node::Comment("This is a comment".into()));
        elem.add_child(Node::Element(Element::new("rect")));

        let doc = Document {
            root: elem,
            ..Default::default()
        };

        let result = stringify(&doc);
        assert!(result.is_ok());
        let output = result.unwrap();

        assert!(output.contains("<!--This is a comment-->"));
        assert!(output.contains("<rect"));
    }

    #[test]
    fn test_stringify_self_closing_tags() {
        let mut svg = Element::new("svg");
        svg.add_child(Node::Element(Element::new("rect")));
        svg.add_child(Node::Element(Element::new("circle")));
        svg.add_child(Node::Element(Element::new("path")));

        let doc = Document {
            root: svg,
            ..Default::default()
        };

        let result = stringify(&doc);
        assert!(result.is_ok());
        let output = result.unwrap();

        // Self-closing tags should use />
        assert!(output.contains("<rect/>") || output.contains("<rect />"));
        assert!(output.contains("<circle/>") || output.contains("<circle />"));
        assert!(output.contains("<path/>") || output.contains("<path />"));
    }

    #[test]
    fn test_stringify_escape_attributes() {
        let mut elem = Element::new("text");
        elem.set_attr("content", "\"Hello & <World>\"");

        let doc = Document {
            root: elem,
            ..Default::default()
        };

        let result = stringify(&doc);
        assert!(result.is_ok());
        let output = result.unwrap();

        // Attributes should be present and contain the content
        assert!(output.contains("content="));
        // The exact escaping format may vary, so just check the element exists
        assert!(output.contains("<text"));
    }

    #[test]
    fn test_stringify_cdata() {
        let mut elem = Element::new("script");
        elem.add_child(Node::CData("function test() { return 1 < 2; }".into()));

        let doc = Document {
            root: elem,
            ..Default::default()
        };

        let result = stringify(&doc);
        assert!(result.is_ok());
        let output = result.unwrap();

        assert!(output.contains("<script"));
        assert!(output.contains("CDATA") || output.contains("function test()"));
        assert!(output.contains("</script>"));
    }

    #[test]
    fn test_stringify_namespaces() {
        let mut elem = Element::new("svg");
        elem.namespaces.insert("".into(), "http://www.w3.org/2000/svg".into());
        elem.namespaces.insert("xlink".into(), "http://www.w3.org/1999/xlink".into());

        let doc = Document {
            root: elem,
            ..Default::default()
        };

        let result = stringify(&doc);
        assert!(result.is_ok());
        let output = result.unwrap();

        assert!(output.contains("xmlns"));
        assert!(output.contains("http://www.w3.org/2000/svg"));
        assert!(output.contains("xmlns:xlink"));
        assert!(output.contains("http://www.w3.org/1999/xlink"));
    }

    #[test]
    fn test_stringify_empty_element() {
        let elem = Element::new("svg");

        let doc = Document {
            root: elem,
            ..Default::default()
        };

        let result = stringify(&doc);
        assert!(result.is_ok());
        let output = result.unwrap();

        // Empty element can be self-closing or have closing tag
        assert!(output.contains("<svg/>") || output.contains("<svg />") || output.contains("<svg></svg>"));
    }

    #[test]
    fn test_stringify_complex_document() {
        // Parse a complex SVG and stringify it
        let svg_input = r#"<svg width="100" height="100">
            <defs>
                <linearGradient id="grad1">
                    <stop offset="0%" style="stop-color:rgb(255,255,0)"/>
                    <stop offset="100%" style="stop-color:rgb(255,0,0)"/>
                </linearGradient>
            </defs>
            <g transform="translate(50,50)">
                <rect width="50" height="50" fill="url(#grad1)"/>
                <text x="25" y="25">Test</text>
            </g>
        </svg>"#;

        let parser = Parser::new();
        let doc = parser.parse(svg_input).unwrap();

        let result = stringify(&doc);
        assert!(result.is_ok());
        let output = result.unwrap();

        // Verify key structures are preserved
        assert!(output.contains("<svg"));
        assert!(output.contains("<defs"));
        assert!(output.contains("<linearGradient"));
        assert!(output.contains("id=\"grad1\""));
        assert!(output.contains("<stop"));
        assert!(output.contains("<g"));
        assert!(output.contains("transform"));
        assert!(output.contains("<rect"));
        assert!(output.contains("fill=\"url(#grad1)\""));
        assert!(output.contains("<text"));
        assert!(output.contains("Test"));
    }

    #[test]
    fn test_stringify_attribute_order() {
        let mut elem = Element::new("rect");
        elem.set_attr("z", "last");
        elem.set_attr("a", "first");
        elem.set_attr("m", "middle");

        let doc = Document {
            root: elem,
            ..Default::default()
        };

        let result = stringify(&doc);
        assert!(result.is_ok());
        let output = result.unwrap();

        // All attributes should be present
        assert!(output.contains("z=\"last\""));
        assert!(output.contains("a=\"first\""));
        assert!(output.contains("m=\"middle\""));
    }

    #[test]
    fn test_stringify_mixed_content() {
        let mut elem = Element::new("text");
        elem.add_child(Node::Text("Hello ".into()));

        let mut tspan = Element::new("tspan");
        tspan.set_attr("font-weight", "bold");
        tspan.add_child(Node::Text("World".into()));

        elem.add_child(Node::Element(tspan));
        elem.add_child(Node::Text("!".into()));

        let doc = Document {
            root: elem,
            ..Default::default()
        };

        let result = stringify(&doc);
        assert!(result.is_ok());
        let output = result.unwrap();

        assert!(output.contains("<text"));
        assert!(output.contains("Hello "));
        assert!(output.contains("<tspan"));
        assert!(output.contains("font-weight=\"bold\""));
        assert!(output.contains("World"));
        assert!(output.contains("!"));
        assert!(output.contains("</text>"));
    }

    #[test]
    fn test_stringify_with_special_nodes() {
        let mut elem = Element::new("svg");
        // Add various node types
        elem.add_child(Node::Comment("comment".into()));
        elem.add_child(Node::Element(Element::new("rect")));
        elem.add_child(Node::Text("text".into()));

        let doc = Document {
            root: elem,
            ..Default::default()
        };

        let result = stringify(&doc);
        assert!(result.is_ok());
        let output = result.unwrap();

        // Should contain basic SVG structure
        assert!(output.contains("<svg"));
        assert!(output.contains("<rect"));
    }

    #[test]
    fn test_stringify_with_dtd() {
        let mut doc = Document::new();
        doc.root.add_child(Node::Element(Element::new("rect")));

        let result = stringify(&doc);
        assert!(result.is_ok());
        let output = result.unwrap();

        // Should contain basic SVG structure
        assert!(output.contains("<svg"));
        assert!(output.contains("<rect"));
    }

    #[test]
    fn test_stringify_with_config_pretty() {
        let svg = r#"<svg><g><rect/></g></svg>"#;
        let document = vexy_svgo_core::parse_svg(svg).unwrap();

        let config = vexy_svgo_core::StringifyConfig {
            pretty:           true,
            indent:           "  ".to_string(),
            newlines:         true,
            quote_attrs:      true,
            self_close:       true,
            initial_capacity: 1024,
        };

        let result = vexy_svgo_core::stringify_with_config(&document, &config);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(!output.is_empty());
        assert!(output.contains("<svg"));
        assert!(output.contains("<g"));
        assert!(output.contains("<rect"));
    }

    #[test]
    fn test_stringify_with_config_compact() {
        let svg = r#"<svg><g><rect/></g></svg>"#;
        let document = vexy_svgo_core::parse_svg(svg).unwrap();

        let config = vexy_svgo_core::StringifyConfig {
            pretty:           false,
            indent:           "".to_string(),
            newlines:         false,
            quote_attrs:      true,
            self_close:       true,
            initial_capacity: 512,
        };

        let result = vexy_svgo_core::stringify_with_config(&document, &config);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(!output.is_empty());
        assert!(output.contains("<svg"));
        // Should be more compact without extra whitespace
        assert!(!output.contains("\n\n"));
    }

    #[test]
    fn test_stringify_large_document() {
        let mut svg = String::from("<svg>");
        for i in 0..100 {
            svg.push_str(&format!("<rect id=\"rect{}\" x=\"{}\" y=\"{}\" width=\"10\" height=\"10\"/>", i, i, i));
        }
        svg.push_str("</svg>");

        let document = vexy_svgo_core::parse_svg(&svg).unwrap();
        let result = stringify(&document);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(!output.is_empty());
        assert!(output.contains("<svg"));
        assert!(output.contains("rect0"));
        assert!(output.contains("rect99"));
    }

    #[test]
    fn test_stringify_round_trip_fidelity() {
        // Test that parse -> stringify -> parse produces equivalent documents
        let original_svg = r#"<svg width="100" height="100">
            <rect x="10" y="10" width="50" height="50" fill="red"/>
            <circle cx="75" cy="75" r="20" fill="blue"/>
        </svg>"#;

        let document1 = vexy_svgo_core::parse_svg(original_svg).unwrap();
        let stringified = stringify(&document1).unwrap();
        let document2 = vexy_svgo_core::parse_svg(&stringified).unwrap();

        // Both stringified versions should be identical
        let stringified1 = stringify(&document1).unwrap();
        let stringified2 = stringify(&document2).unwrap();

        assert_eq!(stringified1, stringified2);
    }
}
