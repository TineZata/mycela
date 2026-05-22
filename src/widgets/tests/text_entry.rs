use super::*;
use crate::channel::ChannelValue;
use crate::config::WidgetType;
use crate::test_helpers as th;

fn w() -> WidgetConfig { th::widget("te", WidgetType::TextEntry) }

#[test]
fn disconnected_state() {
    let html = render_inner_disconnected(&w(), "reason").into_string();
    assert!(html.contains("alarm-disconnected"));
    assert!(html.contains("--"));
}

#[test]
fn no_alarm_class() {
    let cv = ChannelValue { value_str: "50.0".to_string(), ..ChannelValue::default() };
    let html = render_inner_connected(&w(), &cv).into_string();
    assert!(html.contains("alarm-none"), "got: {html}");
}

#[test]
fn minor_alarm_class() {
    let cv = ChannelValue { alarm_severity: 1, ..ChannelValue::default() };
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
fn value_appears_in_input() {
    let cv = ChannelValue { value_str: "12.34".to_string(), ..ChannelValue::default() };
    let html = render_inner_connected(&w(), &cv).into_string();
    assert!(html.contains("12.34"));
}

#[test]
fn string_data_type_uses_text_input() {
    let mut w = w();
    w.data_type = Some("string".to_string());
    let cv = ChannelValue { value_str: "hello".to_string(), ..ChannelValue::default() };
    let html = render_inner_connected(&w, &cv).into_string();
    assert!(html.contains("type=\"text\"") || html.contains("type='text'"));
}
