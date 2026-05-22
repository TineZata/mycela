use super::*;
use crate::channel::ChannelValue;
use crate::config::WidgetType;
use crate::test_helpers as th;

fn w() -> WidgetConfig { th::widget("tb", WidgetType::ToggleButton) }

#[test]
fn disconnected_is_disabled() {
    let html = render_inner_disconnected(&w()).into_string();
    assert!(html.contains("disabled"));
}

#[test]
fn value_above_half_renders_on_state() {
    let cv = ChannelValue { raw_value: 1.0, ..ChannelValue::default() };
    let html = render_inner_connected(&w(), &cv).into_string();
    assert!(html.contains("widget-toggle-btn--on"));
    assert!(html.contains("ON"));
}

#[test]
fn value_zero_renders_off_state() {
    let cv = ChannelValue { raw_value: 0.0, ..ChannelValue::default() };
    let html = render_inner_connected(&w(), &cv).into_string();
    assert!(html.contains("widget-toggle-btn--off"));
    assert!(html.contains("OFF"));
}

#[test]
fn next_value_is_inverse_of_current() {
    // When ON (1.0), clicking should send 0
    let cv_on  = ChannelValue { raw_value: 1.0, ..ChannelValue::default() };
    let html_on = render_inner_connected(&w(), &cv_on).into_string();
    // maud HTML-escapes quotes, so "0" becomes &quot;0&quot;
    assert!(html_on.contains("&quot;0&quot;") || html_on.contains(r#""0""#),
            "expected next_val=0 in: {html_on}");

    // When OFF (0.0), clicking should send 1
    let cv_off = ChannelValue { raw_value: 0.0, ..ChannelValue::default() };
    let html_off = render_inner_connected(&w(), &cv_off).into_string();
    assert!(html_off.contains("&quot;1&quot;") || html_off.contains(r#""1""#),
            "expected next_val=1 in: {html_off}");
}
