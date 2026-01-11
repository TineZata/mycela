use maud::{html, Markup};
use crate::config::WidgetConfig;
use crate::pv_monitor::PvValue;

/// Text update widget - read-only PV value display
pub fn render_text_update(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    tracing::debug!("render_text_update called for widget: {} with value: {:?}", widget.id, value.is_some());
    
    let (alarm_class, icon_html, display_class) = if let Some(v) = value {
        tracing::debug!("Connection status: {:?}, Alarm severity: {}", v.connection_status, v.alarm_severity);
        let class_name = super::alarm_severity_class(v.alarm_severity);
        let icon = super::get_status_icon(&v.connection_status, v.alarm_severity);
        (class_name, icon, format!("pv-display {}", class_name))
    } else {
        ("alarm-disconnected", Some(super::OFFLINE_SVG), "pv-display alarm-disconnected".to_string())
    };
    
    let current_value = value
        .map(|v| {
            let prec = v.precision.unwrap_or(2);
            v.value.to_display_string(Some(prec))
        })
        .unwrap_or_else(|| "--".to_string());
    
    let units = value
        .and_then(|v| v.units.as_deref())
        .unwrap_or("");
    
    let tooltip_text = value.map(|v| super::generate_tooltip(v)).unwrap_or_default();
    
    html! {
        div class={"widget text-update " (alarm_class)} 
            data-widget-id=(widget.id) 
            data-pv=(widget.pv_name)
            title=(tooltip_text) {
            
            label class="widget-label" { (widget.label) }
            
            div class="text-update-container" {
                span class="text-update-display" {
                    @if let Some(icon) = icon_html {
                        img class="display-icon" src=(icon) alt="status";
                    }
                    span {
                        (current_value)
                        @if !units.is_empty() {
                            " " span class="units" { (units) }
                        }
                    }
                }
            }
            
            @if let Some(desc) = &widget.description {
                @if !desc.is_empty() {
                    p class="widget-description" { (desc) }
                }
            }
        }
    }
}
