// this_file: crates/core/src/parser/config.rs

//! Configuration for SVG optimization and parsing

use serde::{Deserialize, Serialize};

/// Configuration for streaming parser
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Buffer size for reading (default: 64KB)
    pub buffer_size: usize,
    /// Maximum nesting depth to prevent stack overflow (default: 256)
    pub max_depth: usize,
    /// Enable lazy loading for large documents
    pub lazy_loading: bool,
    /// Threshold for large text nodes (bytes)
    pub large_text_threshold: Option<usize>,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            buffer_size: 64 * 1024, // 64KB
            max_depth: 256,
            lazy_loading: false,
            large_text_threshold: None,
        }
    }
}

/// Main configuration structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Multipass optimization
    pub multipass: bool,

    /// Pretty print output
    pub pretty: bool,

    /// Indent string for pretty printing
    pub indent: String,

    /// Plugin configurations
    pub plugins: Vec<PluginConfig>,

    /// JS2SVG output options
    pub js2svg: Js2SvgOptions,

    /// Data URI format (if specified)
    pub datauri: Option<DataUriFormat>,

    /// Parallel plugin execution (number of threads)
    #[serde(skip)]
    pub parallel: Option<usize>,

    /// File path (used by CLI)
    #[serde(skip)]
    pub path: Option<String>,
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginConfig {
    /// Just the plugin name (enabled with default params)
    Name(String),

    /// Plugin with parameters
    WithParams {
        name: String,
        #[serde(default)]
        params: serde_json::Value,
    },
}

/// Output formatting options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Js2SvgOptions {
    /// Pretty print output
    pub pretty: bool,

    /// Indent string
    pub indent: String,

    /// Use short tags
    pub use_short_tags: bool,

    /// Final newline
    pub final_newline: bool,

    /// Line ending style
    pub eol: LineEnding,
}

impl Default for Js2SvgOptions {
    fn default() -> Self {
        Self {
            pretty: false,
            indent: "  ".to_string(),
            use_short_tags: true,
            final_newline: true,
            eol: LineEnding::Lf,
        }
    }
}

/// Line ending options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LineEnding {
    /// Unix line endings (\n)
    #[serde(alias = "lf")]
    Lf,
    /// Windows line endings (\r\n)
    #[serde(alias = "crlf")]
    Crlf,
    /// Mac line endings (\r)
    #[serde(alias = "cr")]
    Cr,
}

/// Data URI format options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataUriFormat {
    /// Base64 encoding
    #[serde(alias = "base64")]
    Base64,
    /// URL encoding
    #[serde(alias = "enc")]
    Enc,
    /// Raw text
    #[serde(alias = "unenc")]
    Unenc,
}

use crate::error::VexyError;
use jsonschema::JSONSchema;
use serde_json::Value;

impl Config {
    /// Create a new config with default preset
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a new config with default preset
    pub fn with_default_preset() -> Self {
        let mut config = Self::default();
        
        // Add SVGO default plugins in the correct order
        config.plugins = vec![
            PluginConfig::Name("removeDoctype".to_string()),
            PluginConfig::Name("removeXMLProcInst".to_string()),
            PluginConfig::Name("removeComments".to_string()),
            PluginConfig::Name("removeMetadata".to_string()),
            PluginConfig::Name("removeEditorsNSData".to_string()),
            // Note: cleanupAttrs not implemented yet
            PluginConfig::Name("mergeStyles".to_string()),
            PluginConfig::Name("inlineStyles".to_string()),
            PluginConfig::Name("minifyStyles".to_string()),
            PluginConfig::Name("cleanupIds".to_string()),
            PluginConfig::Name("removeUselessDefs".to_string()),
            // Note: cleanupNumericValues not implemented yet
            PluginConfig::Name("convertColors".to_string()),
            PluginConfig::Name("removeUnknownsAndDefaults".to_string()),
            PluginConfig::Name("removeNonInheritableGroupAttrs".to_string()),
            PluginConfig::Name("removeUselessStrokeAndFill".to_string()),
            // Note: cleanupEnableBackground not implemented yet
            PluginConfig::Name("removeHiddenElems".to_string()),
            PluginConfig::Name("removeEmptyText".to_string()),
            PluginConfig::Name("convertShapeToPath".to_string()),
            PluginConfig::Name("convertEllipseToCircle".to_string()),
            PluginConfig::Name("moveElemsAttrsToGroup".to_string()),
            PluginConfig::Name("moveGroupAttrsToElems".to_string()),
            PluginConfig::Name("collapseGroups".to_string()),
            PluginConfig::Name("convertPathData".to_string()),
            PluginConfig::Name("convertTransform".to_string()),
            PluginConfig::Name("removeEmptyAttrs".to_string()),
            PluginConfig::Name("removeEmptyContainers".to_string()),
            PluginConfig::Name("removeUnusedNS".to_string()),
            PluginConfig::Name("mergePaths".to_string()),
            PluginConfig::Name("sortAttrs".to_string()),
            PluginConfig::Name("sortDefsChildren".to_string()),
            PluginConfig::Name("removeDesc".to_string()),
        ];
        
        config
    }

    /// Load config from a file
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, VexyError> {
        let content = std::fs::read_to_string(path).map_err(VexyError::Io)?;
        let config: Value = serde_json::from_str(&content)
            .map_err(|e| VexyError::Config(e.to_string()))?;

        let schema_content = include_str!("../../../../svgo.schema.json");
        let schema: Value = serde_json::from_str(schema_content)
            .map_err(|e| VexyError::Config(e.to_string()))?;
        let compiled_schema = JSONSchema::compile(&schema)
            .map_err(|e| VexyError::Config(e.to_string()))?;

        let result = compiled_schema.validate(&config);
        if let Err(errors) = result {
            for error in errors {
                eprintln!("Configuration error: {}", error);
            }
            return Err(VexyError::Config("Invalid configuration".to_string()));
        }

        serde_json::from_value(config.clone())
            .map_err(|e| VexyError::Config(e.to_string()))
    }

    /// Get a plugin configuration by name
    pub fn get_plugin(&self, name: &str) -> Option<&PluginConfig> {
        self.plugins.iter().find(|p| p.name() == name)
    }

    /// Add a plugin configuration
    pub fn add_plugin(&mut self, plugin: PluginConfig) {
        self.plugins.push(plugin);
    }

    /// Set plugin enabled/disabled status
    pub fn set_plugin_enabled(&mut self, name: &str, enabled: bool) {
        for plugin in &mut self.plugins {
            if plugin.name() == name {
                match plugin {
                    PluginConfig::Name(plugin_name) => {
                        if !enabled {
                            // Convert to WithParams variant to disable
                            *plugin = PluginConfig::WithParams {
                                name: plugin_name.clone(),
                                params: serde_json::json!({"enabled": false}),
                            };
                        }
                    }
                    PluginConfig::WithParams { params, .. } => {
                        if let Some(obj) = params.as_object_mut() {
                            obj.insert("enabled".to_string(), serde_json::json!(enabled));
                        } else {
                            *params = serde_json::json!({"enabled": enabled});
                        }
                    }
                }
                return;
            }
        }
    }

    /// Configure plugin parameters
    pub fn configure_plugin(&mut self, name: &str, params: serde_json::Value) {
        for plugin in &mut self.plugins {
            if plugin.name() == name {
                match plugin {
                    PluginConfig::Name(plugin_name) => {
                        *plugin = PluginConfig::WithParams {
                            name: plugin_name.clone(),
                            params,
                        };
                    }
                    PluginConfig::WithParams { params: existing_params, .. } => {
                        *existing_params = params;
                    }
                }
                return;
            }
        }
        // If plugin not found, add it with params
        self.plugins.push(PluginConfig::WithParams {
            name: name.to_string(),
            params,
        });
    }
}

impl PluginConfig {
    /// Create a new plugin config
    pub fn new(name: String) -> Self {
        Self::Name(name)
    }

    /// Get the plugin name
    pub fn name(&self) -> &str {
        match self {
            Self::Name(name) => name,
            Self::WithParams { name, .. } => name,
        }
    }

    /// Get the plugin parameters
    pub fn params(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Name(_) => None,
            Self::WithParams { params, .. } => Some(params),
        }
    }

    /// Get mutable plugin parameters, converting to WithParams variant if needed
    pub fn params_mut(&mut self) -> &mut serde_json::Value {
        match self {
            Self::Name(name) => {
                let name = name.clone();
                *self = Self::WithParams {
                    name,
                    params: serde_json::json!({}),
                };
                match self {
                    Self::WithParams { params, .. } => params,
                    _ => unreachable!(),
                }
            }
            Self::WithParams { params, .. } => params,
        }
    }
}

/// Load configuration from a directory (looks for .vexy_svgorc, vexy_svgo.config.json, etc.)
pub fn load_config_from_directory<P: AsRef<std::path::Path>>(
    dir: P,
) -> Result<Config, VexyError> {
    let dir = dir.as_ref();

    // Try various config file names
    let config_files = [
        ".vexy_svgorc",
        ".vexy_svgorc.json",
        "vexy_svgo.config.json",
        ".svgorc",
        ".svgorc.json",
        "svgo.config.json",
    ];

    for filename in &config_files {
        let config_path = dir.join(filename);
        if config_path.exists() {
            return Config::from_file(config_path).map_err(|e| VexyError::Config(e.to_string()));
        }
    }

    // Return default preset with all standard plugins if no config file found
    Ok(Config::with_default_preset())
}
