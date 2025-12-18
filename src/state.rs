use serde::{Deserialize, Serialize};

// Alarm status enum for visual indication
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlarmStatus {
    Normal,
    Minor,
    Major,
    Invalid,
    NotConnected,
}

impl Default for AlarmStatus {
    fn default() -> Self {
        Self::NotConnected
    }
}

impl AlarmStatus {
    /// Convert EPICS severity integer to AlarmStatus
    pub fn from_severity(severity: Option<i32>) -> Self {
        match severity {
            Some(0) => AlarmStatus::Normal,
            Some(1) => AlarmStatus::Minor,
            Some(2) => AlarmStatus::Major,
            Some(3) => AlarmStatus::Invalid,
            _ => AlarmStatus::Normal,
        }
    }
}

// Configuration for the TextEntry component
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextEntryConfigState {
    pub has_border: bool,
    pub has_left_icon: bool,
    pub has_right_icon: bool,
    pub has_units: bool,
    pub units_text: String,
    pub alarm_status: AlarmStatus,
    pub pv_name: String,
    pub current_value: Option<f64>,
}

impl Default for TextEntryConfigState {
    fn default() -> Self {
        Self {
            has_border: true,
            has_left_icon: false,
            has_right_icon: false,
            has_units: false,
            units_text: "mm".to_string(),
            alarm_status: AlarmStatus::NotConnected,
            pv_name: String::new(),
            current_value: None,
        }
    }
}

impl TextEntryConfigState {
    pub fn new(pv_name: &str) -> Self {
        Self {
            pv_name: pv_name.to_string(),
            ..Default::default()
        }
    }
    
    pub fn with_units(mut self, units: &str) -> Self {
        self.has_units = true;
        self.units_text = units.to_string();
        self
    }
    
    pub fn update_from_pv(&mut self, value: f64, severity: Option<i32>) {
        self.current_value = Some(value);
        self.alarm_status = AlarmStatus::from_severity(severity);
    }
    
    pub fn set_disconnected(&mut self) {
        self.alarm_status = AlarmStatus::NotConnected;
        self.current_value = None;
    }
}

// Widget state for Figma-like editor
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WidgetState {
    pub id: String,
    pub widget_type: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub pv_name: String,
    pub label: String,
}

impl Default for WidgetState {
    fn default() -> Self {
        Self {
            id: String::new(),
            widget_type: "text_entry".to_string(),
            x: 0.0,
            y: 0.0,
            width: 300.0,
            height: 50.0,
            pv_name: String::new(),
            label: "Text Entry".to_string(),
        }
    }
}
