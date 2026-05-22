use super::*;
use crate::channel::ChannelValue;
use crate::config::WidgetType;
use crate::test_helpers as th;

fn w() -> WidgetConfig { th::widget("tu", WidgetType::TextUpdate) }

#[test]
fn disconnected_state() {
    let html = render_inner_disconnected(&w(), "reason").into_string();
    assert!(html.contains("alarm-disconnected"));
    assert!(html.contains("--"));
}

#[test]
fn no_alarm_class() {
    let cv = ChannelValue { value_str: "42.0".to_string(), ..ChannelValue::default() };
    let html = render_inner_connected(&w(), &cv).into_string();
    assert!(html.contains("alarm-none"), "got: {html}");
    assert!(html.contains("42.0"));
}

#[test]
fn minor_alarm_class() {
    let cv = ChannelValue { alarm_severity: 1, value_str: "5.0".to_string(), ..ChannelValue::default() };
    let html = render_inner_connected(&w(), &cv).into_string();
    assert!(html.contains("alarm-minor"));
}

#[test]
fn major_alarm_class() {
    let cv = ChannelValue { alarm_severity: 2, ..ChannelValue::default() };
    let html = render_inner_connected(&w(), &cv).into_string();
    assert!(html.contains("alarm-major"));
}

#[test]
fn value_and_units_rendered() {
    let cv = ChannelValue {
        value_str: "99.9".to_string(),
        units: "degC".to_string(),
        ..ChannelValue::default()
    };
    let html = render_inner_connected(&w(), &cv).into_string();
    assert!(html.contains("99.9"));
    assert!(html.contains("degC"));
}
