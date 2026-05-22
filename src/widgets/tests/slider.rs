use super::*;
use crate::channel::ChannelValue;
use crate::config::WidgetType;
use crate::test_helpers as th;

fn w() -> WidgetConfig { th::widget("sl", WidgetType::Slider) }

#[test]
fn disconnected_state() {
    let html = render_inner_disconnected(&w()).into_string();
    // Disconnected shows offline icon and disables the input
    assert!(html.contains("widget-status-icon"), "{html}");
    assert!(html.contains("disabled"));
}

#[test]
fn no_alarm_class() {
    let cv = ChannelValue { value_str: "50.0".to_string(), ..ChannelValue::default() };
    let html = render_inner_connected(&w(), &cv).into_string();
    // No alarm — no alarm icon present
    assert!(!html.contains("widget-status-icon"), "got: {html}");
}

#[test]
fn minor_alarm_class() {
    let cv = ChannelValue { alarm_severity: 1, ..ChannelValue::default() };
    let html = render_inner_connected(&w(), &cv).into_string();
    assert!(html.contains(crate::widgets::MINOR_ALARM_SVG));
}

#[test]
fn major_alarm_class() {
    let cv = ChannelValue { alarm_severity: 2, ..ChannelValue::default() };
    let html = render_inner_connected(&w(), &cv).into_string();
    assert!(html.contains(crate::widgets::MAJOR_ALARM_SVG));
}

#[test]
fn control_range_in_input() {
    let cv = ChannelValue {
        control_low: 10.0, control_high: 90.0,
        raw_value: 50.0, value_str: "50.0".to_string(),
        ..ChannelValue::default()
    };
    let html = render_inner_connected(&w(), &cv).into_string();
    assert!(html.contains("10") && html.contains("90"),
            "expected range in output, got: {html}");
}
