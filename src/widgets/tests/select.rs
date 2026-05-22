use super::*;
use crate::channel::ChannelValue;
use crate::config::WidgetType;
use crate::test_helpers as th;

fn w() -> WidgetConfig { th::widget("sel", WidgetType::Select) }

#[test]
fn disconnected_shows_disconnected_class() {
    let html = render_inner_disconnected(&w()).into_string();
    assert!(html.contains("alarm-disconnected"));
}

#[test]
fn no_alarm_class() {
    let cv = ChannelValue { enum_index: 0, enum_choices: vec![], ..ChannelValue::default() };
    let html = render_inner_connected(&w(), &cv).into_string();
    assert!(html.contains("widget-select alarm-none"), "got: {html}");
}

#[test]
fn minor_alarm_class() {
    let cv = ChannelValue { alarm_severity: 1, ..ChannelValue::default() };
    let html = render_inner_connected(&w(), &cv).into_string();
    assert!(html.contains("widget-select alarm-minor"));
}

#[test]
fn major_alarm_class() {
    let cv = ChannelValue { alarm_severity: 2, ..ChannelValue::default() };
    let html = render_inner_connected(&w(), &cv).into_string();
    assert!(html.contains("widget-select alarm-major"));
}

#[test]
fn enum_choices_rendered_as_options() {
    let cv = ChannelValue {
        enum_index: 1,
        enum_choices: vec!["Auto".to_string(), "Manual".to_string(), "Off".to_string()],
        ..ChannelValue::default()
    };
    let html = render_inner_connected(&w(), &cv).into_string();
    assert!(html.contains("Auto"));
    assert!(html.contains("Manual"));
    assert!(html.contains("Off"));
}

#[test]
fn selected_index_is_current_enum_index() {
    let cv = ChannelValue {
        enum_index: 2,
        enum_choices: vec!["A".to_string(), "B".to_string(), "C".to_string()],
        ..ChannelValue::default()
    };
    let html = render_inner_connected(&w(), &cv).into_string();
    // Option at index 2 should have 'selected'
    assert!(html.contains("selected"));
}
