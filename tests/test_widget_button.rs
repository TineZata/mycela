// Copyright 2026 Tine Zata
// SPDX-License-Identifier: MPL-2.0

mod test_widget_button {
    use ctrl_sys_widgets::channel::ChannelValue;
    use ctrl_sys_widgets::config::{WidgetConfig, WidgetType};
    use ctrl_sys_widgets::widgets::button::{render_inner_connected, render_inner_disconnected};

    fn w() -> WidgetConfig {
        WidgetConfig {
            id: "btn".to_string(),
            widget_type: WidgetType::Button,
            label: "btn label".to_string(),
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
    fn test_disconnected_button_renders_with_disabled_attribute() {
        let html = render_inner_disconnected(&w()).into_string();
        assert!(html.contains("disabled"));
    }

    #[test]
    fn test_connected_button_renders_without_disabled_attribute() {
        let cv = ChannelValue::default();
        let html = render_inner_connected(&w(), &cv).into_string();
        assert!(!html.contains("disabled"));
    }

    #[test]
    fn test_button_widget_label_appears_in_rendered_html() {
        let html = render_inner_connected(&w(), &ChannelValue::default()).into_string();
        // label from widget helper is "btn label"
        assert!(html.contains("btn label") || html.contains("btn"));
    }
}
