//! Comprehensive unit tests for the AST module

#[cfg(test)]
mod tests {
    use vexy_svgo_core::ast::*;

    #[test]
    fn test_element_creation() {
        let elem = Element::new("rect");
        assert_eq!(elem.name, "rect");
        assert!(elem.attributes.is_empty());
        assert!(elem.children.is_empty());
        assert!(elem.namespaces.is_empty());
    }

    #[test]
    fn test_element_with_attributes() {
        let mut elem = Element::new("rect");
        elem.set_attr("x", "10");
        elem.set_attr("y", "20");
        elem.set_attr("width", "100");
        elem.set_attr("height", "50");

        assert_eq!(elem.attr("x"), Some("10"));
        assert_eq!(elem.attr("y"), Some("20"));
        assert_eq!(elem.attr("width"), Some("100"));
        assert_eq!(elem.attr("height"), Some("50"));
        assert_eq!(elem.attr("non-existent"), None);
    }

    #[test]
    fn test_element_children() {
        let mut parent = Element::new("g");
        let child1 = Element::new("rect");
        let child2 = Element::new("circle");

        parent.add_child(Node::Element(child1));
        parent.add_child(Node::Element(child2));

        assert_eq!(parent.children.len(), 2);

        if let Node::Element(rect) = &parent.children[0] {
            assert_eq!(rect.name, "rect");
        } else {
            panic!("Expected rect element");
        }

        if let Node::Element(circle) = &parent.children[1] {
            assert_eq!(circle.name, "circle");
        } else {
            panic!("Expected circle element");
        }
    }

    #[test]
    fn test_text_node() {
        let text = Node::Text("Hello World".into());

        match &text {
            Node::Text(content) => assert_eq!(content.as_ref(), "Hello World"),
            _ => panic!("Expected text node"),
        }
    }

    #[test]
    fn test_comment_node() {
        let comment = Node::Comment("This is a comment".into());

        match &comment {
            Node::Comment(content) => assert_eq!(content.as_ref(), "This is a comment"),
            _ => panic!("Expected comment node"),
        }
    }

    #[test]
    fn test_cdata_node() {
        let cdata = Node::CData("<![CDATA[Some data]]>".into());

        match &cdata {
            Node::CData(content) => assert_eq!(content.as_ref(), "<![CDATA[Some data]]>"),
            _ => panic!("Expected CDATA node"),
        }
    }

    #[test]
    fn test_document_creation() {
        let doc = Document::new();
        assert_eq!(doc.root.name, "svg");
        assert!(doc.root.children.is_empty());
    }

    #[test]
    fn test_document_with_content() {
        let mut doc = Document::new();
        let rect = Element::new("rect");
        doc.root.add_child(Node::Element(rect));

        assert_eq!(doc.root.children.len(), 1);
    }

    #[test]
    fn test_element_namespaces() {
        let mut elem = Element::new("svg");
        elem.namespaces.insert("".into(), "http://www.w3.org/2000/svg".into());
        elem.namespaces.insert("xlink".into(), "http://www.w3.org/1999/xlink".into());

        assert_eq!(elem.namespaces.get("").map(|s| s.as_ref()), Some("http://www.w3.org/2000/svg"));
        assert_eq!(elem.namespaces.get("xlink").map(|s| s.as_ref()), Some("http://www.w3.org/1999/xlink"));
    }

    #[test]
    fn test_element_has_attr() {
        let mut elem = Element::new("rect");
        elem.set_attr("x", "10");

        assert!(elem.has_attr("x"));
        assert!(!elem.has_attr("y"));
    }

    #[test]
    fn test_element_remove_attr() {
        let mut elem = Element::new("rect");
        elem.set_attr("x", "10");
        elem.set_attr("y", "20");

        assert!(elem.has_attr("x"));
        assert!(elem.has_attr("y"));

        elem.remove_attr("x");

        assert!(!elem.has_attr("x"));
        assert!(elem.has_attr("y"));
    }

    #[test]
    fn test_element_clone() {
        let mut elem = Element::new("rect");
        elem.set_attr("x", "10");
        elem.set_attr("y", "20");
        elem.add_child(Node::Text("content".into()));

        let cloned = elem.clone();

        assert_eq!(cloned.name, elem.name);
        assert_eq!(cloned.attr("x"), elem.attr("x"));
        assert_eq!(cloned.attr("y"), elem.attr("y"));
        assert_eq!(cloned.children.len(), elem.children.len());
    }

    #[test]
    fn test_element_memory_usage() {
        let elem = Element::new("rect");
        let memory = elem.estimated_memory_usage();

        // Basic element should have some memory usage
        assert!(memory > 0);

        // Add some attributes and children to test memory calculation
        let mut complex_elem = Element::new("g");
        complex_elem.set_attr("id", "group1");
        complex_elem.set_attr("transform", "translate(100, 200)");

        for i in 0..10 {
            let mut child = Element::new("rect");
            child.set_attr("x", &i.to_string());
            complex_elem.add_child(Node::Element(child));
        }

        let complex_memory = complex_elem.estimated_memory_usage();
        assert!(complex_memory > memory);
    }

    #[test]
    fn test_node_equality() {
        let text1 = Node::Text("Hello".into());
        let text2 = Node::Text("Hello".into());
        let text3 = Node::Text("World".into());

        assert_eq!(text1, text2);
        assert_ne!(text1, text3);

        let elem1 = Node::Element(Element::new("rect"));
        let elem2 = Node::Element(Element::new("rect"));
        let elem3 = Node::Element(Element::new("circle"));

        // Elements with same name are equal if they have same structure
        assert_eq!(elem1, elem2); // Same structure
        assert_ne!(elem1, elem3); // Different names
    }

    #[test]
    fn test_element_deep_hierarchy() {
        let mut root = Element::new("svg");
        let mut group = Element::new("g");
        let mut subgroup = Element::new("g");
        let rect = Element::new("rect");

        subgroup.add_child(Node::Element(rect));
        group.add_child(Node::Element(subgroup));
        root.add_child(Node::Element(group));

        assert_eq!(root.children.len(), 1);

        // Navigate through the hierarchy
        if let Node::Element(g1) = &root.children[0] {
            assert_eq!(g1.name, "g");
            if let Node::Element(g2) = &g1.children[0] {
                assert_eq!(g2.name, "g");
                if let Node::Element(r) = &g2.children[0] {
                    assert_eq!(r.name, "rect");
                } else {
                    panic!("Expected rect at depth 3");
                }
            } else {
                panic!("Expected g at depth 2");
            }
        } else {
            panic!("Expected g at depth 1");
        }
    }

    #[test]
    fn test_mixed_content() {
        let mut elem = Element::new("text");
        elem.add_child(Node::Text("Hello ".into()));

        let mut tspan = Element::new("tspan");
        tspan.set_attr("font-weight", "bold");
        tspan.add_child(Node::Text("World".into()));

        elem.add_child(Node::Element(tspan));
        elem.add_child(Node::Text("!".into()));

        assert_eq!(elem.children.len(), 3);

        // Verify mixed content structure
        match &elem.children[0] {
            Node::Text(t) => assert_eq!(t.as_ref(), "Hello "),
            _ => panic!("Expected text node"),
        }

        match &elem.children[1] {
            Node::Element(e) => {
                assert_eq!(e.name, "tspan");
                assert_eq!(e.attr("font-weight"), Some("bold"));
            },
            _ => panic!("Expected element node"),
        }

        match &elem.children[2] {
            Node::Text(t) => assert_eq!(t.as_ref(), "!"),
            _ => panic!("Expected text node"),
        }
    }
}
