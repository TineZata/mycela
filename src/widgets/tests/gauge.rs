use super::*;
use crate::channel::ChannelValue;
use crate::config::WidgetType;
use crate::test_helpers as th;

fn gauge_widget() -> WidgetConfig { th::widget("g", WidgetType::Gauge) }

#[test]
fn disconnected_renders_offline_icon() {
    let html = render_inner_disconnected(&gauge_widget()).into_string();
    // Disconnected gauge shows the offline SVG icon and "--" placeholder
    assert!(html.contains("widget-status-icon"), "got: {html}");
    assert!(html.contains("--"));
}

#[test]
fn connected_no_alarm_shows_minor_alarm_svg_absent() {
    let cv = ChannelValue { value_str: "42.5".to_string(), alarm_severity: 0,
        display_low: 0.0, display_high: 100.0, ..ChannelValue::default() };
    let html = render_inner_connected(&gauge_widget(), &cv).into_string();
    assert!(!html.contains(crate::widgets::MINOR_ALARM_SVG));
    assert!(!html.contains(crate::widgets::MAJOR_ALARM_SVG));
}

#[test]
fn connected_minor_alarm_shows_minor_icon() {
    let cv = ChannelValue { alarm_severity: 1, ..ChannelValue::default() };
    let html = render_inner_connected(&gauge_widget(), &cv).into_string();
    assert!(html.contains(crate::widgets::MINOR_ALARM_SVG));
}

#[test]
fn connected_major_alarm_shows_major_icon() {
    let cv = ChannelValue { alarm_severity: 2, ..ChannelValue::default() };
    let html = render_inner_connected(&gauge_widget(), &cv).into_string();
    assert!(html.contains(crate::widgets::MAJOR_ALARM_SVG));
}

#[test]
fn gauge_fill_width_reflects_percentage() {
    let cv = ChannelValue {
        raw_value: 50.0, display_low: 0.0, display_high: 100.0,
        ..ChannelValue::default()
    };
    let html = render_inner_connected(&gauge_widget(), &cv).into_string();
    assert!(html.contains("width: 50.0%") || html.contains("width:50"),
            "expected 50% fill, got: {html}");
}

#[test]
fn alarm_markers_present_when_limits_non_default() {
    let cv = ChannelValue {
        raw_value: 50.0, display_low: 0.0, display_high: 100.0,
        low_alarm_limit: 10.0, low_warn_limit: 20.0,
        high_warn_limit: 80.0, high_alarm_limit: 90.0,
        ..ChannelValue::default()
    };
    let html = render_inner_connected(&gauge_widget(), &cv).into_string();
    assert!(html.contains("gauge-marker--alarm"));
    assert!(html.contains("gauge-marker--warn"));
}

#[test]
fn alarm_markers_absent_when_limits_at_default() {
    // Default ChannelValue has low_alarm/warn=0 and high_warn/alarm=100
    // which the gauge treats as "not configured"
    let cv = ChannelValue { display_low: 0.0, display_high: 100.0, ..ChannelValue::default() };
    let html = render_inner_connected(&gauge_widget(), &cv).into_string();
    assert!(!html.contains("gauge-marker--alarm"));
    assert!(!html.contains("gauge-marker--warn"));
}

#[test]
fn outer_render_contains_data_ch_and_widget_id() {
    let w = gauge_widget();
    let html = render_gauge(&w).into_string();
    assert!(html.contains("data-widget-id=\"g\""));
}
