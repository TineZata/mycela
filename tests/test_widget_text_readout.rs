// Copyright 2026 Tine Zata
// SPDX-License-Identifier: MPL-2.0

mod test_widget_text_readout {
    use mycela::channel::ChannelValue;
    use mycela::config::{WidgetConfig, WidgetType};
    use mycela::widgets::text_update::{render_inner_connected, render_inner_disconnected};

    fn w() -> WidgetConfig {
        WidgetConfig {
            id: "tu".to_string(),
            widget_type: WidgetType::TextUpdate,
            label: "Text Update".to_string(),
            protocol: None,
            data_type: None,
            description: None,
            style: None,
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
    fn test_disconnected_text_update_shows_alarm_disconnected_class_and_placeholder() {
        let html = render_inner_disconnected(&w(), "reason").into_string();
        assert!(html.contains("alarm-disconnected"));
        assert!(html.contains("--"));
    }

    #[test]
    fn test_connected_text_update_with_no_alarm_uses_alarm_none_class_and_displays_value() {
        let cv = ChannelValue { value_str: "42.0".to_string(), ..ChannelValue::default() };
        let html = render_inner_connected(&w(), &cv).into_string();
        assert!(html.contains("alarm-none"), "got: {html}");
        assert!(html.contains("42.0"));
    }

    #[test]
    fn test_connected_text_update_with_minor_alarm_uses_alarm_minor_class() {
        let cv = ChannelValue {
            alarm_severity: 1,
            value_str: "5.0".to_string(),
            ..ChannelValue::default()
        };
        let html = render_inner_connected(&w(), &cv).into_string();
        assert!(html.contains("alarm-minor"));
    }

    #[test]
    fn test_connected_text_update_with_major_alarm_uses_alarm_major_class() {
        let cv = ChannelValue { alarm_severity: 2, ..ChannelValue::default() };
        let html = render_inner_connected(&w(), &cv).into_string();
        assert!(html.contains("alarm-major"));
    }

    #[test]
    fn test_connected_text_update_renders_value_string_and_units() {
        let cv = ChannelValue {
            value_str: "99.9".to_string(),
            units: "degC".to_string(),
            ..ChannelValue::default()
        };
        let html = render_inner_connected(&w(), &cv).into_string();
        assert!(html.contains("99.9"));
        assert!(html.contains("degC"));
    }
}
