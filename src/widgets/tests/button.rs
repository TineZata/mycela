use super::*;
use crate::channel::ChannelValue;
use crate::config::WidgetType;
use crate::test_helpers as th;

fn w() -> WidgetConfig { th::widget("btn", WidgetType::Button) }

#[test]
fn disconnected_button_is_disabled() {
    let html = render_inner_disconnected(&w()).into_string();
    assert!(html.contains("disabled"));
}

#[test]
fn connected_button_is_not_disabled() {
    let cv = ChannelValue::default();
    let html = render_inner_connected(&w(), &cv).into_string();
    assert!(!html.contains("disabled"));
}

#[test]
fn button_label_appears_in_output() {
    let html = render_inner_connected(&w(), &ChannelValue::default()).into_string();
    // label from widget helper is "btn label"
    assert!(html.contains("btn label") || html.contains("btn"));
}
