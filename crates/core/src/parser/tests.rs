// this_file: crates/core/src/parser/tests.rs

//! Unit tests for the SVG parser module

#[cfg(test)]
mod parser_tests {
    use std::borrow::Cow;

    use crate::ast::Node;
    use crate::parser::main::Parser;

    #[test]
    fn test_parser_creation() {
        let parser = Parser::new();
        let result = parser.parse("<svg></svg>");
        assert!(result.is_ok(), "Parser::new() should create a working parser");
    }

    #[test]
    fn test_parse_simple_svg() {
        let svg = r#"<svg><rect width="100" height="50"/></svg>"#;
        let parser = Parser::new();
        let result = parser.parse(svg);

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.root.name.as_ref(), "svg");
        assert_eq!(doc.root.children.len(), 1);

        if let Node::Element(rect) = &doc.root.children[0] {
            assert_eq!(rect.name.as_ref(), "rect");
            assert_eq!(rect.attributes.get("width"), Some(&Cow::Borrowed("100")));
            assert_eq!(rect.attributes.get("height"), Some(&Cow::Borrowed("50")));
        } else {
            panic!("Expected element node");
        }
    }

    #[test]
    fn test_parse_with_attributes() {
        let svg = r#"<svg width="200" height="100" viewBox="0 0 200 100">
            <circle cx="50" cy="50" r="40" fill="red"/>
        </svg>"#;

        let parser = Parser::new();
        let result = parser.parse(svg);

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.root.attributes.get("width"), Some(&Cow::Borrowed("200")));
        assert_eq!(doc.root.attributes.get("height"), Some(&Cow::Borrowed("100")));
        assert_eq!(doc.root.attributes.get("viewBox"), Some(&Cow::Borrowed("0 0 200 100")));
    }

    #[test]
    fn test_parse_with_comments() {
        let svg = r#"<svg><!-- This is a comment --><rect/></svg>"#;

        let parser = Parser::new().preserve_comments(true);
        let result = parser.parse(svg);

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.root.children.len(), 2);

        if let Node::Comment(comment) = &doc.root.children[0] {
            assert_eq!(comment.as_ref(), " This is a comment ");
        } else {
            panic!("Expected comment node");
        }
    }

    #[test]
    fn test_parse_nested_elements() {
        let svg = r#"<svg>
            <g id="group1">
                <rect x="0" y="0" width="50" height="50"/>
                <circle cx="25" cy="25" r="20"/>
            </g>
        </svg>"#;

        let parser = Parser::new();
        let result = parser.parse(svg);

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.root.children.len(), 1);

        if let Node::Element(g) = &doc.root.children[0] {
            assert_eq!(g.name.as_ref(), "g");
            assert_eq!(g.attributes.get("id"), Some(&Cow::Borrowed("group1")));
            assert_eq!(g.children.len(), 2);
        } else {
            panic!("Expected group element");
        }
    }

    #[test]
    fn test_parse_text_content() {
        let svg = r#"<svg><text>Hello World</text></svg>"#;

        let parser = Parser::new();
        let result = parser.parse(svg);

        assert!(result.is_ok());
        let doc = result.unwrap();

        if let Node::Element(text) = &doc.root.children[0] {
            assert_eq!(text.name.as_ref(), "text");
            assert_eq!(text.children.len(), 1);

            if let Node::Text(content) = &text.children[0] {
                assert_eq!(content.as_ref(), "Hello World");
            } else {
                panic!("Expected text node");
            }
        } else {
            panic!("Expected text element");
        }
    }

    #[test]
    fn test_parse_empty_svg() {
        let svg = r#"<svg></svg>"#;

        let parser = Parser::new();
        let result = parser.parse(svg);

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.root.name.as_ref(), "svg");
        assert_eq!(doc.root.children.len(), 0);
        assert_eq!(doc.root.attributes.len(), 0);
    }

    #[test]
    fn test_parse_self_closing_tags() {
        let svg = r#"<svg><rect/><circle/><path/></svg>"#;

        let parser = Parser::new();
        let result = parser.parse(svg);

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.root.children.len(), 3);

        let names: Vec<_> = doc
            .root
            .children
            .iter()
            .filter_map(|child| {
                if let Node::Element(elem) = child {
                    Some(elem.name.as_ref())
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(names, vec!["rect", "circle", "path"]);
    }

    #[test]
    fn test_parse_with_namespace() {
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\">
            <rect xlink:href=\"#target\"/>
        </svg>";

        let parser = Parser::new();
        let result = parser.parse(svg);

        assert!(result.is_ok());
        let doc = result.unwrap();
        // xmlns attributes are stored in the namespaces map, not attributes
        // Default xmlns (no prefix) uses empty string as key
        assert!(doc.root.namespaces.contains_key(""), "Default xmlns should be in namespaces");
        assert_eq!(doc.root.namespaces.get("").map(|s| s.as_ref()), Some("http://www.w3.org/2000/svg"));
        // xmlns:xlink uses "xlink" as key
        assert!(doc.root.namespaces.contains_key("xlink"), "xlink namespace should be in namespaces");
        assert_eq!(doc.root.namespaces.get("xlink").map(|s| s.as_ref()), Some("http://www.w3.org/1999/xlink"));
    }

    #[test]
    fn test_parse_cdata() {
        let svg = r#"<svg><style><![CDATA[
            .red { fill: red; }
            .blue { fill: blue; }
        ]]></style></svg>"#;

        let parser = Parser::new();
        let result = parser.parse(svg);

        assert!(result.is_ok());
        let doc = result.unwrap();

        if let Node::Element(style) = &doc.root.children[0] {
            assert_eq!(style.name.as_ref(), "style");
            assert!(!style.children.is_empty());
        } else {
            panic!("Expected style element");
        }
    }

    #[test]
    fn test_parse_entity_expansion() {
        let svg = r#"<svg><text>&lt;Hello &amp; World&gt;</text></svg>"#;

        let parser = Parser::new();
        let result = parser.parse(svg);

        assert!(result.is_ok());
        let doc = result.unwrap();

        if let Node::Element(text) = &doc.root.children[0] {
            if let Node::Text(content) = &text.children[0] {
                assert_eq!(content.as_ref(), "<Hello & World>");
            } else {
                panic!("Expected text node");
            }
        } else {
            panic!("Expected text element");
        }
    }

    #[test]
    fn test_parse_whitespace_preservation() {
        let svg = r#"<svg>
            <text>  Spaces  </text>
        </svg>"#;

        let parser = Parser::new().preserve_whitespace(true);
        let result = parser.parse(svg);

        assert!(result.is_ok());
        let doc = result.unwrap();

        // With whitespace preservation, we should have text nodes with whitespace
        assert!(doc.root.children.iter().any(|node| { matches!(node, Node::Text(text) if text.contains('\n')) }));
    }

    #[test]
    fn test_parse_invalid_xml() {
        let svg = r#"<svg><rect></svg>"#; // Missing closing tag

        let parser = Parser::new();
        let result = parser.parse(svg);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_special_attributes() {
        let svg = r#"<svg><rect data-foo="bar" aria-label="Rectangle" role="img"/></svg>"#;

        let parser = Parser::new();
        let result = parser.parse(svg);

        assert!(result.is_ok());
        let doc = result.unwrap();

        if let Node::Element(rect) = &doc.root.children[0] {
            assert_eq!(rect.attributes.get("data-foo"), Some(&Cow::Borrowed("bar")));
            assert_eq!(rect.attributes.get("aria-label"), Some(&Cow::Borrowed("Rectangle")));
            assert_eq!(rect.attributes.get("role"), Some(&Cow::Borrowed("img")));
        } else {
            panic!("Expected rect element");
        }
    }
}
