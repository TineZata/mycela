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
    pub data_type: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub style: Option<WidgetStyle>,
    #[serde(default)]
    pub server: Option<ServerConfig>,
}

/// Server configuration for providing a PV
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default)]
    pub alarm_serverity: Option<String>,
    #[serde(default)]
    pub alarm_status: Option<String>,
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
    Gauge,
    Led,
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
