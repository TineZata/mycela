/// Shared helpers used by `#[cfg(test)]` blocks throughout the crate.
/// This module is compiled only in test builds.

use crate::config::{
    AlarmMetadata, ControlMetadata, DisplayMetadata, ModbusTCPConfig,
    ModbusRegisterType, PvMetadata, WidgetConfig, WidgetType,
};

/// Minimal `WidgetConfig` with all optionals set to `None`.
pub fn widget(id: &str, widget_type: WidgetType) -> WidgetConfig {
    WidgetConfig {
        id: id.to_string(),
        widget_type,
        label: format!("{} label", id),
        protocol: None,
        data_type: None,
        description: None,
        style: None,
        options: None,
        orientation: None,
        level: None,
        children: None,
        max_points: None,
        chart_type: None,
        axis_label_x: None,
        axis_label_y: None,
        size: None,
        metadata: None,
    }
}

/// Standard alarm metadata: low/alarm=10/20, high warn/alarm=80/90, MINOR/MAJOR.
pub fn alarm_meta() -> AlarmMetadata {
    AlarmMetadata {
        low_alarm_limit: 10.0,
        low_warning_limit: 20.0,
        high_warning_limit: 80.0,
        high_alarm_limit: 90.0,
        low_alarm_severity: "MAJOR".to_string(),
        low_warning_severity: "MINOR".to_string(),
        high_warning_severity: "MINOR".to_string(),
        high_alarm_severity: "MAJOR".to_string(),
        hysteresis: 1,
    }
}

/// `PvMetadata` with all three sections populated.
pub fn full_metadata() -> PvMetadata {
    PvMetadata {
        display: Some(DisplayMetadata {
            limit_low: 0.0,
            limit_high: 100.0,
            description: "Test channel".to_string(),
            precision: 2,
            units: "degC".to_string(),
        }),
        control: Some(ControlMetadata {
            limit_low: 5.0,
            limit_high: 95.0,
            min_step: 0.1,
        }),
        alarm: Some(alarm_meta()),
    }
}

/// `ModbusTCPConfig` with 1:1 scale (physical == raw).
pub fn modbus_cfg(scale: f64, offset: f64) -> ModbusTCPConfig {
    ModbusTCPConfig {
        host: "127.0.0.1".to_string(),
        port: 502,
        unit_id: 1,
        register: 1000,
        register_type: ModbusRegisterType::HoldingRegister,
        min_poll_interval_ms: 500,
        scale,
        offset,
        word_count: 1,
    }
}
