use serde::{Deserialize, Serialize};
use std::fs;

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
    pub description: Option<String>,
    #[serde(default)]
    pub style: Option<WidgetStyle>,
}

/// Widget type enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WidgetType {
    TextEntry,
    Gauge,
    LED,
    Button,
    Slider,
    Chart,
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
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: ScreenConfig = serde_json::from_str(&content)?;
        Ok(config)
    }
    
    /// Save screen configuration to JSON file
    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}
