use super::*;
use crate::config::{ControlMetadata, DisplayMetadata, PvMetadata, WidgetType};
use crate::test_helpers as th;

// ── build_channel_value: value formatting ──────────────────────────────

#[test]
fn float_value_uses_precision() {
    let cv = build_channel_value(3.14159, &th::modbus_cfg(1.0, 0.0), &th::widget("w", WidgetType::Gauge));
    assert_eq!(cv.value_str, "3.14"); // default precision = 2
}

#[test]
fn int32_data_type_truncates_to_integer_string() {
    let mut w = th::widget("w", WidgetType::TextUpdate);
    w.data_type = Some("int32".to_string());
    let cv = build_channel_value(42.9, &th::modbus_cfg(1.0, 0.0), &w);
    assert_eq!(cv.value_str, "42");
}

#[test]
fn int_data_type_also_truncates() {
    let mut w = th::widget("w", WidgetType::TextUpdate);
    w.data_type = Some("int".to_string());
    let cv = build_channel_value(7.7, &th::modbus_cfg(1.0, 0.0), &w);
    assert_eq!(cv.value_str, "7");
}

// ── build_channel_value: display / control ranges ──────────────────────

#[test]
fn no_metadata_display_range_derived_from_register() {
    let m = th::modbus_cfg(1.0, 0.0); // offset=0 → display_low=0
    let cv = build_channel_value(50.0, &m, &th::widget("w", WidgetType::Gauge));
    assert!((cv.display_low - 0.0).abs() < f64::EPSILON);
    assert!(cv.display_high > 0.0); // 65535 * 1.0 + 0 = 65535
}

#[test]
fn display_metadata_overrides_range() {
    let mut w = th::widget("w", WidgetType::Gauge);
    w.metadata = Some(PvMetadata {
        display: Some(DisplayMetadata {
            limit_low: 0.0, limit_high: 200.0,
            description: String::new(), precision: 1, units: "bar".to_string(),
        }),
        control: None, alarm: None,
    });
    let cv = build_channel_value(100.0, &th::modbus_cfg(1.0, 0.0), &w);
    assert_eq!(cv.units, "bar");
    assert_eq!(cv.precision, 1);
    assert!((cv.display_high - 200.0).abs() < f64::EPSILON);
    assert_eq!(cv.value_str, "100.0");
}

#[test]
fn control_metadata_overrides_control_range() {
    let mut w = th::widget("w", WidgetType::Slider);
    w.metadata = Some(PvMetadata {
        display: Some(DisplayMetadata {
            limit_low: 0.0, limit_high: 100.0,
            description: String::new(), precision: 2, units: String::new(),
        }),
        control: Some(ControlMetadata { limit_low: 10.0, limit_high: 90.0, min_step: 1.0 }),
        alarm: None,
    });
    let cv = build_channel_value(50.0, &th::modbus_cfg(1.0, 0.0), &w);
    assert!((cv.control_low  - 10.0).abs() < f64::EPSILON);
    assert!((cv.control_high - 90.0).abs() < f64::EPSILON);
}

#[test]
fn control_falls_back_to_display_when_absent() {
    let mut w = th::widget("w", WidgetType::Slider);
    w.metadata = Some(PvMetadata {
        display: Some(DisplayMetadata {
            limit_low: 5.0, limit_high: 95.0,
            description: String::new(), precision: 2, units: String::new(),
        }),
        control: None, alarm: None,
    });
    let cv = build_channel_value(50.0, &th::modbus_cfg(1.0, 0.0), &w);
    assert!((cv.control_low  - cv.display_low ).abs() < f64::EPSILON);
    assert!((cv.control_high - cv.display_high).abs() < f64::EPSILON);
}

// ── build_channel_value: alarm severity computation ────────────────────

#[test]
fn no_alarm_metadata_severity_zero() {
    let cv = build_channel_value(50.0, &th::modbus_cfg(1.0, 0.0), &th::widget("w", WidgetType::Gauge));
    assert_eq!(cv.alarm_severity, 0);
}

#[test]
fn alarm_severity_zero_in_normal_range() {
    let mut w = th::widget("w", WidgetType::Gauge);
    w.metadata = Some(PvMetadata { display: None, control: None, alarm: Some(th::alarm_meta()) });
    let cv = build_channel_value(50.0, &th::modbus_cfg(1.0, 0.0), &w);
    assert_eq!(cv.alarm_severity, 0);
}

#[test]
fn alarm_severity_minor_above_high_warning() {
    let mut w = th::widget("w", WidgetType::Gauge);
    w.metadata = Some(PvMetadata { display: None, control: None, alarm: Some(th::alarm_meta()) });
    let cv = build_channel_value(85.0, &th::modbus_cfg(1.0, 0.0), &w);
    assert_eq!(cv.alarm_severity, 1);
}

#[test]
fn alarm_severity_major_above_high_alarm() {
    let mut w = th::widget("w", WidgetType::Gauge);
    w.metadata = Some(PvMetadata { display: None, control: None, alarm: Some(th::alarm_meta()) });
    let cv = build_channel_value(95.0, &th::modbus_cfg(1.0, 0.0), &w);
    assert_eq!(cv.alarm_severity, 2);
}

#[test]
fn alarm_severity_minor_below_low_warning() {
    let mut w = th::widget("w", WidgetType::Gauge);
    w.metadata = Some(PvMetadata { display: None, control: None, alarm: Some(th::alarm_meta()) });
    let cv = build_channel_value(15.0, &th::modbus_cfg(1.0, 0.0), &w);
    assert_eq!(cv.alarm_severity, 1);
}

#[test]
fn alarm_severity_major_below_low_alarm() {
    let mut w = th::widget("w", WidgetType::Gauge);
    w.metadata = Some(PvMetadata { display: None, control: None, alarm: Some(th::alarm_meta()) });
    let cv = build_channel_value(5.0, &th::modbus_cfg(1.0, 0.0), &w);
    assert_eq!(cv.alarm_severity, 2);
}

#[test]
fn alarm_limits_are_copied_to_channel_value() {
    let mut w = th::widget("w", WidgetType::Gauge);
    w.metadata = Some(PvMetadata { display: None, control: None, alarm: Some(th::alarm_meta()) });
    let cv = build_channel_value(50.0, &th::modbus_cfg(1.0, 0.0), &w);
    assert!((cv.low_alarm_limit  - 10.0).abs() < f64::EPSILON);
    assert!((cv.low_warn_limit   - 20.0).abs() < f64::EPSILON);
    assert!((cv.high_warn_limit  - 80.0).abs() < f64::EPSILON);
    assert!((cv.high_alarm_limit - 90.0).abs() < f64::EPSILON);
}

// ── build_channel_value: raw_value passthrough ─────────────────────────

#[test]
fn raw_value_is_the_physical_argument() {
    let cv = build_channel_value(37.5, &th::modbus_cfg(1.0, 0.0), &th::widget("w", WidgetType::Gauge));
    assert!((cv.raw_value - 37.5).abs() < f64::EPSILON);
}
