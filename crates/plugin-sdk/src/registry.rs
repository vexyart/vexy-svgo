// this_file: crates/plugin-sdk/src/registry.rs

//! Plugin registry with migrated plugins
//!
//! This module provides a registry pre-populated with all migrated plugins
//! using the new visitor-based architecture.

use crate::plugins::*;
use vexy_svgo_core::plugin_registry::PluginRegistry;

/// Create a plugin registry with all migrated plugins
pub fn create_migrated_plugin_registry() -> PluginRegistry {
    let mut registry = PluginRegistry::new();

    // Register all migrated plugins using the new API
    registry.register("removeComments", || RemoveCommentsPlugin::new());
    registry.register("removeEmptyAttrs", || RemoveEmptyAttrsPlugin::new());
    registry.register("removeUselessDefs", || RemoveUselessDefsPlugin::new());
    registry.register("collapseGroups", || CollapseGroupsPlugin::new());
    registry.register("moveElemsAttrsToGroup", || MoveElemsAttrsToGroupPlugin::new());
    registry.register("moveGroupAttrsToElems", || MoveGroupAttrsToElemsPlugin::new());
    registry.register("removeUnknownsAndDefaults", || RemoveUnknownsAndDefaultsPlugin::new());
    registry.register("convertColors", || ConvertColorsPlugin::new());
    registry.register("removeViewBox", || RemoveViewBoxPlugin::new());
    registry.register("mergePaths", || MergePathsPlugin::new());
    registry.register("inlineStyles", || InlineStylesPlugin::new());
    registry.register("cleanupIds", || CleanupIdsPlugin::new());
    registry.register("convertStyleToAttrs", || ConvertStyleToAttrsPlugin::new());
    registry.register("removeEmptyContainers", || RemoveEmptyContainersPlugin::new());
    registry.register("removeHiddenElems", || RemoveHiddenElemsPlugin::new());
    registry.register("removeEditorsNSData", || RemoveEditorsNSDataPlugin::new());
    registry.register("removeElementsByAttr", || RemoveElementsByAttrPlugin::new());
    registry.register("removeUnusedNS", || RemoveUnusedNSPlugin::new());
    registry.register("cleanupAttrs", || CleanupAttrsPlugin::new());
    registry.register("cleanupEnableBackground", || CleanupEnableBackgroundPlugin::new());
    // registry.register("cleanupListOfValues", || CleanupListOfValuesPlugin::new()); // Not implemented yet
    registry.register("mergeStyles", || MergeStylesPlugin::new());
    registry.register("removeDoctype", || RemoveDoctypePlugin::new());
    registry.register("removeDimensions", || RemoveDimensionsPlugin::new());
    registry.register("removeXMLProcInst", || RemoveXMLProcInstPlugin::new());
    registry.register("removeMetadata", || RemoveMetadataPlugin::new());
    registry.register("removeEmptyText", || RemoveEmptyTextPlugin::new());
    registry.register("convertEllipseToCircle", || ConvertEllipseToCirclePlugin::new());
    registry.register("convertOneStopGradients", || ConvertOneStopGradientsPlugin::new());
    registry.register("convertShapeToPath", || ConvertShapeToPathPlugin::new());
    registry.register("convertPathData", || ConvertPathDataPlugin::new());
    registry.register("convertTransform", || ConvertTransformPlugin::new());
    registry.register("applyTransforms", || ApplyTransformsPlugin::new());
    registry.register("cleanupNumericValues", || CleanupNumericValuesPlugin::new());
    registry.register("minifyStyles", || MinifyStylesPlugin::new());
    registry.register("removeNonInheritableGroupAttrs", || RemoveNonInheritableGroupAttrsPlugin::new());
    registry.register("sortAttrs", || SortAttrsPlugin::new());
    registry.register("sortDefsChildren", || SortDefsChildrenPlugin::new());
    registry.register("removeTitle", || RemoveTitlePlugin::new());
    registry.register("removeDesc", || RemoveDescPlugin::new());
    registry.register("addAttributesToSVGElement", || AddAttributesToSVGElementPlugin::new());
    registry.register("addClassesToSVGElement", || AddClassesToSVGElementPlugin::new());
    registry.register("removeScripts", || RemoveScriptsPlugin::new());
    registry.register("removeStyleElement", || RemoveStyleElementPlugin::new());
    registry.register("removeRasterImages", || RemoveRasterImagesPlugin::new());
    registry.register("removeOffCanvasPaths", || RemoveOffCanvasPathsPlugin::new());
    registry.register("removeAttrs", || RemoveAttrsPlugin::new());
    registry.register("removeDeprecatedAttrs", || RemoveDeprecatedAttrsPlugin::new());
    registry.register("removeUselessTransforms", || RemoveUselessTransformsPlugin::new());
    registry.register("removeUselessStrokeAndFill", || RemoveUselessStrokeAndFillPlugin::new());
    registry.register("removeXlink", || RemoveXlinkPlugin::new());
    registry.register("removeXMLNS", || RemoveXmlnsPlugin::new());
    registry.register("prefixIds", || PrefixIdsPlugin::new());
    registry.register("reusePaths", || ReusePathsPlugin::new());
    registry.register("removeAttributesBySelector", || RemoveAttributesBySelectorPlugin::new());
    registry.register("usvg", || UsvgPlugin::new());

    registry
}

/// Get the default plugin configuration for migrated plugins
pub fn get_default_plugin_configs() -> Vec<vexy_svgo_core::parser::config::PluginConfig> {
    use serde_json::json;
    use vexy_svgo_core::parser::config::PluginConfig;

    vec![
        PluginConfig::WithParams {
            name: "removeComments".to_string(),
            params: json!({"preservePatterns": true}),
        },
        PluginConfig::Name("removeEmptyAttrs".to_string()),
        PluginConfig::Name("removeUselessDefs".to_string()),
        PluginConfig::Name("collapseGroups".to_string()),
        PluginConfig::Name("moveGroupAttrsToElems".to_string()),
        PluginConfig::WithParams {
            name: "removeUnknownsAndDefaults".to_string(),
            params: json!({
                "unknownContent": true,
                "unknownAttrs": true,
                "defaultAttrs": true,
                "defaultMarkupDeclarations": true,
                "uselessOverrides": true,
                "keepDataAttrs": true,
                "keepAriaAttrs": true,
                "keepRoleAttr": false
            }),
        },
        PluginConfig::WithParams {
            name: "convertColors".to_string(),
            params: json!({
                "currentColor": false,
                "names2hex": true,
                "rgb2hex": true,
                "convertCase": "lower",
                "shorthex": true,
                "shortname": true
            }),
        },
        PluginConfig::Name("removeViewBox".to_string()),
        PluginConfig::WithParams {
            name: "mergePaths".to_string(),
            params: json!({
                "force": false,
                "floatPrecision": 3,
                "noSpaceAfterFlags": false
            }),
        },
        PluginConfig::WithParams {
            name: "inlineStyles".to_string(),
            params: json!({
                "onlyMatchedOnce": true,
                "removeMatchedSelectors": true,
                "useMqs": true,
                "usePseudos": true
            }),
        },
        PluginConfig::WithParams {
            name: "cleanupIds".to_string(),
            params: json!({
                "remove": true,
                "minify": true,
                "preserve": [],
                "preservePrefixes": [],
                "force": false
            }),
        },
        PluginConfig::WithParams {
            name: "convertStyleToAttrs".to_string(),
            params: json!({
                "keepImportant": false
            }),
        },
        PluginConfig::Name("removeEmptyContainers".to_string()),
        PluginConfig::WithParams {
            name: "removeHiddenElems".to_string(),
            params: json!({
                "displayNone": true,
                "opacity0": true,
                "circleR0": true,
                "ellipseRX0": true,
                "ellipseRY0": true,
                "rectWidth0": true,
                "rectHeight0": true,
                "patternWidth0": true,
                "patternHeight0": true,
                "imageWidth0": true,
                "imageHeight0": true,
                "pathEmptyD": true,
                "polylineEmptyPoints": true,
                "polygonEmptyPoints": true
            }),
        },
        PluginConfig::WithParams {
            name: "removeEditorsNSData".to_string(),
            params: json!({
                "additionalNamespaces": []
            }),
        },
        PluginConfig::WithParams {
            name: "removeElementsByAttr".to_string(),
            params: json!({"enabled": false}),
        },
        PluginConfig::Name("removeUnusedNS".to_string()),
        PluginConfig::WithParams {
            name: "cleanupAttrs".to_string(),
            params: json!({
                "newlines": true,
                "trim": true,
                "spaces": true
            }),
        },
        PluginConfig::Name("cleanupEnableBackground".to_string()),
        PluginConfig::Name("mergeStyles".to_string()),
        PluginConfig::Name("removeDoctype".to_string()),
        PluginConfig::WithParams {
            name: "removeDimensions".to_string(),
            params: json!({"enabled": false}),
        },
        PluginConfig::Name("removeXMLProcInst".to_string()),
        PluginConfig::Name("removeMetadata".to_string()),
        PluginConfig::WithParams {
            name: "removeEmptyText".to_string(),
            params: json!({
                "text": true,
                "tspan": true,
                "tref": true
            }),
        },
        PluginConfig::Name("convertEllipseToCircle".to_string()),
        PluginConfig::WithParams {
            name: "convertOneStopGradients".to_string(),
            params: json!({"enabled": false}),
        },
        PluginConfig::WithParams {
            name: "convertShapeToPath".to_string(),
            params: json!({
                "convertArcs": false,
                "floatPrecision": null
            }),
        },
        PluginConfig::WithParams {
            name: "convertPathData".to_string(),
            params: json!({
                "floatPrecision": 3,
                "transformPrecision": 5,
                "removeUseless": true,
                "collapseRepeated": true,
                "utilizeAbsolute": true,
                "leadingZero": true,
                "negativeExtraSpace": true
            }),
        },
        PluginConfig::WithParams {
            name: "convertTransform".to_string(),
            params: json!({
                "convertToShorts": true,
                "floatPrecision": 3,
                "transformPrecision": 5,
                "matrixToTransform": true,
                "shortTranslate": true,
                "shortScale": true,
                "shortRotate": true,
                "removeUseless": true,
                "collapseIntoOne": true,
                "leadingZero": true,
                "negativeExtraSpace": false
            }),
        },
        PluginConfig::WithParams {
            name: "cleanupNumericValues".to_string(),
            params: json!({
                "floatPrecision": 3,
                "leadingZero": true,
                "defaultPx": true,
                "convertToPx": true
            }),
        },
        PluginConfig::WithParams {
            name: "minifyStyles".to_string(),
            params: json!({
                "restructure": true,
                "forceMediaMerge": false,
                "comments": false,
                "usage": null
            }),
        },
        PluginConfig::Name("removeNonInheritableGroupAttrs".to_string()),
        PluginConfig::WithParams {
            name: "sortAttrs".to_string(),
            params: json!({
                "order": ["id", "width", "height", "x", "x1", "x2", "y", "y1", "y2", "cx", "cy", "r", "fill", "stroke", "marker", "d", "points"],
                "xmlnsOrder": "front"
            }),
        },
        PluginConfig::Name("sortDefsChildren".to_string()),
        PluginConfig::Name("removeTitle".to_string()),
        PluginConfig::Name("removeDesc".to_string()),
        PluginConfig::WithParams {
            name: "addAttributesToSVGElement".to_string(),
            params: json!({"enabled": false}),
        },
        PluginConfig::WithParams {
            name: "addClassesToSVGElement".to_string(),
            params: json!({"enabled": false}),
        },
        PluginConfig::WithParams {
            name: "removeScripts".to_string(),
            params: json!({"enabled": false}),
        },
        PluginConfig::WithParams {
            name: "removeStyleElement".to_string(),
            params: json!({"enabled": false}),
        },
        PluginConfig::WithParams {
            name: "removeRasterImages".to_string(),
            params: json!({"enabled": false}),
        },
        PluginConfig::WithParams {
            name: "removeOffCanvasPaths".to_string(),
            params: json!({"enabled": false}),
        },
        PluginConfig::WithParams {
            name: "removeAttrs".to_string(),
            params: json!({"enabled": false}),
        },
        PluginConfig::WithParams {
            name: "removeDeprecatedAttrs".to_string(),
            params: json!({"enabled": false}),
        },
        PluginConfig::Name("removeUselessTransforms".to_string()),
        PluginConfig::Name("removeUselessStrokeAndFill".to_string()),
        PluginConfig::WithParams {
            name: "removeXlink".to_string(),
            params: json!({
                "enabled": false,
                "includeLegacy": true
            }),
        },
        PluginConfig::WithParams {
            name: "removeXMLNS".to_string(),
            params: json!({"enabled": false}),
        },
        PluginConfig::WithParams {
            name: "prefixIds".to_string(),
            params: json!({"enabled": false}),
        },
        PluginConfig::WithParams {
            name: "reusePaths".to_string(),
            params: json!({"enabled": false}),
        },
        PluginConfig::WithParams {
            name: "removeAttributesBySelector".to_string(),
            params: json!({"enabled": false}),
        },
    ]
}

/// Get plugin names for all migrated plugins
pub fn get_migrated_plugin_names() -> Vec<&'static str> {
    vec![
        "removeComments",
        "removeEmptyAttrs",
        "removeUselessDefs",
        "collapseGroups",
        "moveGroupAttrsToElems",
        "removeUnknownsAndDefaults",
        "convertColors",
        "removeViewBox",
        "mergePaths",
        "inlineStyles",
        "cleanupIds",
        "convertStyleToAttrs",
        "removeEmptyContainers",
        "removeHiddenElems",
        "removeEditorsNSData",
        "removeElementsByAttr",
        "removeUnusedNS",
        "cleanupAttrs",
        "cleanupEnableBackground",
        "mergeStyles",
        "removeDoctype",
        "removeDimensions",
        "removeXMLProcInst",
        "removeMetadata",
        "removeEmptyText",
        "convertEllipseToCircle",
        "convertOneStopGradients",
        "convertShapeToPath",
        "convertPathData",
        "convertTransform",
        "cleanupNumericValues",
        "minifyStyles",
        "removeNonInheritableGroupAttrs",
        "sortAttrs",
        "sortDefsChildren",
        "removeTitle",
        "removeDesc",
        "addAttributesToSVGElement",
        "addClassesToSVGElement",
        "removeScripts",
        "removeStyleElement",
        "removeRasterImages",
        "removeOffCanvasPaths",
        "removeAttrs",
        "removeDeprecatedAttrs",
        "removeUselessTransforms",
        "removeUselessStrokeAndFill",
        "removeXlink",
        "removeXMLNS",
        "prefixIds",
        "reusePaths",
        "removeAttributesBySelector",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_migrated_plugin_registry() {
        let registry = create_migrated_plugin_registry();

        // Check that all migrated plugins are registered
        for plugin_name in get_migrated_plugin_names() {
            assert!(
                registry.create_plugin(plugin_name).is_some(),
                "Plugin {} should be registered",
                plugin_name
            );
        }
    }

    #[test]
    fn test_get_default_plugin_configs() {
        let configs = get_default_plugin_configs();

        // Check that we have configs for all migrated plugins
        assert_eq!(configs.len(), get_migrated_plugin_names().len());

        // Check that we have a config for each plugin name
        let config_names: Vec<&str> = configs.iter().map(|c| c.name()).collect();
        for plugin_name in get_migrated_plugin_names() {
            assert!(
                config_names.contains(&plugin_name),
                "Plugin {} should have a config",
                plugin_name
            );
        }
    }

    #[test]
    fn test_apply_migrated_plugins() {
        let registry = create_migrated_plugin_registry();
        let configs = get_default_plugin_configs();

        // Create a test document
        let mut doc = vexy_svgo_core::ast::Document::new();

        // Apply all plugins - should not error
        let result = registry.apply_plugins(&mut doc, &configs);
        assert!(result.is_ok(), "Applying migrated plugins should succeed");
    }

    #[test]
    fn test_plugin_parameter_validation() {
        let registry = create_migrated_plugin_registry();

        // Test valid parameters
        let valid_config = vexy_svgo_core::parser::config::PluginConfig::WithParams {
            name: "removeComments".to_string(),
            params: json!({"preservePatterns": false}),
        };

        let mut doc = vexy_svgo_core::ast::Document::new();
        let result = registry.apply_plugin(&mut doc, &valid_config);
        assert!(result.is_ok(), "Valid parameters should be accepted");

        // Test invalid parameters
        let invalid_config = vexy_svgo_core::parser::config::PluginConfig::WithParams {
            name: "removeComments".to_string(),
            params: json!({"preservePatterns": "invalid"}),
        };

        let mut doc2 = vexy_svgo_core::ast::Document::new();
        let result = registry.apply_plugin(&mut doc2, &invalid_config);
        assert!(result.is_err(), "Invalid parameters should be rejected");
    }
}
