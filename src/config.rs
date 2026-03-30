use serde::{Deserialize, Serialize};
use std::fs;
use std::fmt;

/// Custom error type for configuration loading
#[derive(Debug)]
pub enum ConfigError {
    FileError(std::io::Error),
    JsonError { source: serde_json::Error, context: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::FileError(e) => write!(f, "Failed to read config file: {}", e),
            ConfigError::JsonError { source, context } => {
                write!(f, "Configuration JSON error: {}\n{}", source, context)
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::FileError(err)
    }
}

/// Screen configuration loaded from JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenConfig {
    pub id: String,
    pub title: String,
    pub description: String,
    pub widgets: Vec<WidgetConfig>,
}

/// Individual widget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetConfig {
    pub id: String,
    pub pv_name: String,
    #[serde(rename = "type")]
    pub widget_type: WidgetType,
    pub label: String,
    #[serde(default)]
    pub data_type: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub style: Option<WidgetStyle>,
    #[serde(default)]
    pub server: Option<ServerConfig>,
    /// Enum choice labels for Select widgets backed by enum PVs
    #[serde(default)]
    pub options: Option<Vec<String>>,
    /// Gauge orientation: "horizontal" (default) or "vertical"
    #[serde(default)]
    pub orientation: Option<String>,
}

/// Server configuration for providing a PV
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default)]
    pub alarm_serverity: Option<String>,
    #[serde(default)]
    pub alarm_status: Option<String>,
    #[serde(default)]
    pub alarm_message: Option<String>,
    #[serde(default)]
    pub metadata: Option<PvMetadata>,
}

/// PV metadata configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PvMetadata {
    #[serde(default)]
    pub display: Option<DisplayMetadata>,
    #[serde(default)]
    pub control: Option<ControlMetadata>,
    #[serde(default)]
    pub alarm: Option<AlarmMetadata>,
}

/// Display metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayMetadata {
    pub limit_low: f64,
    pub limit_high: f64,
    pub description: String,
    pub precision: i32,
    pub units: String,
}

/// Control metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlMetadata {
    pub limit_low: f64,
    pub limit_high: f64,
    #[serde(default)]
    pub min_step: f64,
}

/// Alarm metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlarmMetadata {
    pub low_alarm_limit: f64,
    pub low_warning_limit: f64,
    pub high_alarm_limit: f64,
    pub high_warning_limit: f64,
    pub low_alarm_severity: String,
    pub low_warning_severity: String,
    pub high_warning_severity: String,
    pub high_alarm_severity: String,
    pub hysteresis: i32,
}

/// Widget type enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WidgetType {
    TextEntry,
    TextUpdate,
    Gauge,
    Led,
    Button,
    ToggleButton,
    Slider,
    Chart,
    Select,
}

/// Optional widget styling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetStyle {
    #[serde(default)]
    pub width: Option<String>,
    #[serde(default)]
    pub height: Option<String>,
    #[serde(default)]
    pub background: Option<String>,
}

impl ScreenConfig {
    /// Load screen configuration from JSON file
    pub fn load(path: &str) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        
        match serde_json::from_str::<ScreenConfig>(&content) {
            Ok(config) => {
                // Validate the config has required fields populated correctly
                Self::validate_config(&config)?;
                Ok(config)
            }
            Err(e) => {
                let context = Self::build_error_context(&e, &content, path);
                Err(ConfigError::JsonError { source: e, context })
            }
        }
    }
    
    /// Validate that the configuration has all required data
    fn validate_config(config: &ScreenConfig) -> Result<(), ConfigError> {
        // Check for any additional runtime validations if needed
        // For example, checking that widget IDs are unique
        let mut seen_ids = std::collections::HashSet::new();
        for (idx, widget) in config.widgets.iter().enumerate() {
            if !seen_ids.insert(&widget.id) {
                let context = format!(
                    "⚠️  Widget #{} has duplicate ID: '{}'\n\
                     💡 Each widget must have a unique 'id' field.",
                    idx + 1, widget.id
                );
                // Create a synthetic error by serializing and deserializing invalid data
                let err = serde_json::from_str::<()>("\"duplicate_id\"")
                    .unwrap_err();
                return Err(ConfigError::JsonError { 
                    source: err,
                    context 
                });
            }
        }
        Ok(())
    }
    
    /// Build a helpful error context message
    fn build_error_context(error: &serde_json::Error, content: &str, path: &str) -> String {
        let mut context = format!("📄 File: {}\n", path);
        
        // Try to determine what's wrong and where
        let line = error.line();
        if line > 0 {
            context.push_str(&format!("📍 Line: {}, Column: {}\n\n", line, error.column()));
            
            // Show the problematic line and surrounding context
            let lines: Vec<&str> = content.lines().collect();
            let start = line.saturating_sub(3);
            let end = (line + 2).min(lines.len());
            
            context.push_str("Context:\n");
            for (i, line_content) in lines[start..end].iter().enumerate() {
                let line_num = start + i + 1;
                if line_num == line {
                    context.push_str(&format!("  ➤ {}: {}\n", line_num, line_content));
                } else {
                    context.push_str(&format!("    {}: {}\n", line_num, line_content));
                }
            }
            context.push_str("\n");
        }
        
        // Add helpful hints based on error message
        let error_msg = error.to_string();
        context.push_str("❌ Error: ");
        context.push_str(&error_msg);
        context.push_str("\n\n");
        
        if error_msg.contains("missing field") {
            if let Some(field_name) = Self::extract_field_name(&error_msg) {
                context.push_str("💡 Hint: ");
                context.push_str(&Self::get_field_hint(&field_name));
                context.push_str("\n");
            }
        } else if error_msg.contains("unknown variant") || error_msg.contains("unknown field") {
            context.push_str("💡 Hint: Check for typos in field names or enum values.\n");
            context.push_str("   Valid widget types: text_entry, text_update, gauge, led, button, slider, chart\n");
        } else if error_msg.contains("invalid type") {
            context.push_str("💡 Hint: Check that the field has the correct data type (string, number, boolean, etc.)\n");
        }
        
        context
    }
    
    /// Extract field name from serde error message
    fn extract_field_name(error_msg: &str) -> Option<String> {
        // Pattern: "missing field `fieldname`"
        if let Some(start) = error_msg.find("missing field `") {
            let start = start + 15; // length of "missing field `"
            if let Some(end) = error_msg[start..].find('`') {
                return Some(error_msg[start..start + end].to_string());
            }
        }
        None
    }
    
    /// Get helpful hint for a missing field
    fn get_field_hint(field_name: &str) -> String {
        match field_name {
            "id" => "Each widget must have a unique 'id' field (string).".to_string(),
            "pv_name" => "Each widget must have a 'pv_name' field specifying the EPICS PV (string).".to_string(),
            "type" => "Each widget must have a 'type' field. Valid types: text_entry, text_update, gauge, led, button, slider, chart".to_string(),
            "label" => "Each widget must have a 'label' field for display (string).".to_string(),
            "title" => "The config root must have a 'title' field (string).".to_string(),
            "description" => "The config root must have a 'description' field (string).".to_string(),
            "widgets" => "The config root must have a 'widgets' array containing widget configurations.".to_string(),
            _ => format!("The field '{}' is required but missing.", field_name),
        }
    }
    
    /// Save screen configuration to JSON file
    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}
