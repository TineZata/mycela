use super::*;
use crate::config::{EpicsPvaConfig, ProtocolConfig, WidgetConfig, WidgetStyle, WidgetType};

fn make_widget(style: Option<WidgetStyle>) -> WidgetConfig {
    WidgetConfig {
        id: "test1".into(),
        widget_type: WidgetType::Gauge,
        label: "Test".into(),
        protocol: Some(ProtocolConfig::EpicsPva(EpicsPvaConfig {
            pv_name: "demo:pv".into(),
            server: None,
            pv_names: None,
        })),
        data_type: None,
        description: None,
        style,
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

#[test]
fn style_none_produces_no_attribute() {
    let w = make_widget(None);
    assert!(widget_container_style(&w).is_none());
    let html = render_gauge(&w).into_string();
    // The outer div should not have a style attribute
    // Extract the opening tag (up to the first '>') and check there
    let outer_tag = &html[..html.find('>').unwrap() + 1];
    assert!(!outer_tag.contains("style="),
            "expected no style on outer div, got: {}", outer_tag);
}

#[test]
fn style_width_height_in_html() {
    let w = make_widget(Some(WidgetStyle {
        width: Some("400px".into()),
        height: Some("200px".into()),
        background: None,
    }));
    let css = widget_container_style(&w).unwrap();
    assert!(css.contains("width:400px;"), "CSS must contain width");
    assert!(css.contains("height:200px;"), "CSS must contain height");

    let html = render_gauge(&w).into_string();
    assert!(html.contains("style=\"width:400px;height:200px;\""),
            "rendered HTML must include inline style, got: {}", html);
}

#[test]
fn style_width_only() {
    let w = make_widget(Some(WidgetStyle {
        width: Some("50%".into()),
        height: None,
        background: None,
    }));
    let css = widget_container_style(&w).unwrap();
    assert_eq!(css, "width:50%;");
    assert!(!css.contains("height"));
}

// ── alarm_severity_class ───────────────────────────────────────────────

#[test]
fn alarm_class_no_alarm() {
    assert_eq!(alarm_severity_class(0), "alarm-none");
}

#[test]
fn alarm_class_minor() {
    assert_eq!(alarm_severity_class(1), "alarm-minor");
}

#[test]
fn alarm_class_major() {
    assert_eq!(alarm_severity_class(2), "alarm-major");
}

#[test]
fn alarm_class_invalid_and_unknown() {
    assert_eq!(alarm_severity_class(3), "alarm-invalid");
    assert_eq!(alarm_severity_class(99), "alarm-invalid");
}

// ── alarm_status_str ───────────────────────────────────────────────────

#[test]
fn alarm_status_known_codes() {
    assert_eq!(alarm_status_str(0), "No Alarm");
    assert_eq!(alarm_status_str(1), "Device");
    assert_eq!(alarm_status_str(2), "Driver");
    assert_eq!(alarm_status_str(6), "Client");
}

#[test]
fn alarm_status_unknown_code() {
    assert_eq!(alarm_status_str(99), "Unknown");
}

// ── collect_data_widgets ───────────────────────────────────────────────

fn simple_widget(id: &str, wtype: WidgetType) -> WidgetConfig {
    WidgetConfig {
        id: id.to_string(), widget_type: wtype, label: id.to_string(),
        protocol: None, data_type: None, description: None, style: None,
        options: None, orientation: None, level: None, children: None,
        max_points: None, chart_type: None, axis_label_x: None,
        axis_label_y: None, size: None,
        metadata: None,
    }
}

#[test]
fn collect_flat_list_unchanged() {
    let ws = vec![
        simple_widget("w1", WidgetType::TextUpdate),
        simple_widget("w2", WidgetType::Gauge),
    ];
    let result = collect_data_widgets(&ws);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].id, "w1");
    assert_eq!(result[1].id, "w2");
}

#[test]
fn collect_skips_group_expands_children() {
    let mut grp = simple_widget("grp", WidgetType::Group);
    grp.children = Some(vec![
        simple_widget("c1", WidgetType::Led),
        simple_widget("c2", WidgetType::Slider),
    ]);
    let ws = vec![simple_widget("top", WidgetType::TextUpdate), grp];
    let result = collect_data_widgets(&ws);
    assert_eq!(result.len(), 3);
    assert!(result.iter().all(|w| w.widget_type != WidgetType::Group));
    assert!(result.iter().any(|w| w.id == "top"));
    assert!(result.iter().any(|w| w.id == "c1"));
    assert!(result.iter().any(|w| w.id == "c2"));
}

#[test]
fn collect_nested_groups_fully_flattened() {
    let mut inner = simple_widget("inner", WidgetType::Group);
    inner.children = Some(vec![simple_widget("deep", WidgetType::Gauge)]);
    let mut outer = simple_widget("outer", WidgetType::Group);
    outer.children = Some(vec![inner]);
    let result = collect_data_widgets(&[outer]);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id, "deep");
}

#[test]
fn collect_empty_input() {
    assert!(collect_data_widgets(&[]).is_empty());
}

#[test]
fn collect_group_with_no_children_contributes_nothing() {
    let grp = simple_widget("empty_grp", WidgetType::Group);
    let result = collect_data_widgets(&[grp]);
    assert!(result.is_empty());
}
