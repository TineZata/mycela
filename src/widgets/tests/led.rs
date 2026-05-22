use super::*;
use crate::channel::ChannelValue;
use crate::config::WidgetType;
use crate::test_helpers as th;

fn w() -> WidgetConfig { th::widget("led", WidgetType::Led) }

#[test]
fn disconnected_shows_offline_icon() {
    let html = render_inner_disconnected(&w()).into_string();
    assert!(html.contains(crate::widgets::OFFLINE_SVG));
}

#[test]
fn value_above_half_is_on() {
    let cv = ChannelValue { raw_value: 1.0, ..ChannelValue::default() };
    let html = render_inner_connected(&w(), &cv).into_string();
    assert!(html.contains("led-on"));
    assert!(html.contains("ON"));
}

#[test]
fn value_below_half_is_off() {
    let cv = ChannelValue { raw_value: 0.0, ..ChannelValue::default() };
    let html = render_inner_connected(&w(), &cv).into_string();
    assert!(html.contains("led-off"));
    assert!(html.contains("OFF"));
}

#[test]
fn minor_alarm_shows_minor_icon() {
    let cv = ChannelValue { alarm_severity: 1, raw_value: 1.0, ..ChannelValue::default() };
    let html = render_inner_connected(&w(), &cv).into_string();
    assert!(html.contains(crate::widgets::MINOR_ALARM_SVG));
}

#[test]
fn major_alarm_shows_major_icon() {
    let cv = ChannelValue { alarm_severity: 2, ..ChannelValue::default() };
    let html = render_inner_connected(&w(), &cv).into_string();
    assert!(html.contains(crate::widgets::MAJOR_ALARM_SVG));
}
