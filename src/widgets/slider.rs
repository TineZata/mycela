use maud::{html, Markup};
use crate::{AppState, config::WidgetConfig};
use crate::pv_monitor::{PvValue, ConnectionStatus};

/// Slider widget - adjustable value
pub fn render_slider(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    let current_value = value.and_then(|v| v.value.as_f64()).unwrap_or(0.0);
    
    // Extract control range from PV metadata or use defaults
    let (min, max) = value
        .and_then(|v| {
            if let (Some(low), Some(high)) = (v.control_low, v.control_high) {
                Some((low, high))
            } else if let (Some(low), Some(high)) = (v.limit_low, v.limit_high) {
                Some((low, high))
            } else {
                None
            }
        })
        .unwrap_or((0.0, 100.0));
    
    // Use min_step from metadata if available
    let step = value.and_then(|v| v.min_step).unwrap_or(0.1);
    
    // Use NTType display method for formatting
    let display_value = value.map(|v| v.value.to_display_string(v.precision)).unwrap_or_else(|| "--".to_string());
    
    let units = value
        .and_then(|v| v.units.as_deref())
        .unwrap_or("");
    
    let alarm_class = value
        .map(|v| super::alarm_severity_class(v.alarm_severity))
        .unwrap_or("alarm-disconnected");
    
    let disabled = !matches!(value.map(|v| &v.connection_status), Some(&ConnectionStatus::Connected));
    
    let icon_html = value.and_then(|v| super::get_status_icon(&v.connection_status, v.alarm_severity));
    
    let tooltip_text = value.map(|v| super::generate_tooltip(v)).unwrap_or_default();
    
    html! {
        div class={"widget slider " (alarm_class)} 
            data-widget-id=(widget.id)
            data-pv=(widget.pv_name)
            title=(tooltip_text) {
            
            label class="widget-label" { 
                (widget.label)
                @if let Some(icon) = icon_html {
                    img class="widget-status-icon" src=(icon) alt="status";
                }
            }
            
            div class="slider-container" {
                input type="range"
                    class="pv-slider"
                    name="value"
                    min=(format!("{}", min))
                    max=(format!("{}", max))
                    step=(format!("{}", step))
                    value=(format!("{}", current_value))
                    disabled[disabled]
                    hx-post={"/api/pv/" (widget.pv_name) "/set"}
                    hx-trigger="change"
                    hx-target="next .slider-value";
                
                span class="slider-value" { 
                    (display_value)
                    @if !units.is_empty() {
                        " " (units)
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

pub async fn render_slider_simple(pv_name: &str, label: &str, state: &AppState) -> Markup {
    let value = state.pv_monitor.get_value(pv_name, ).await;
    let status_class = match value.connection_status {
        ConnectionStatus::Connected => "status-connected",
        _ => "status-disconnected",
    };
    
    html! {
        div class={"slider-widget " (status_class)} {
            label { (label) }
            input type="range" 
                min="-100" max="100" step="1"
                value=(value.value.as_f64().unwrap_or(0.0))
                disabled[!matches!(value.connection_status, ConnectionStatus::Connected)]
                hx-post={"/api/pv/" (pv_name) "/set"}
                hx-trigger="change"
                hx-vals={"js:{value: event.target.value}"};
            span class="slider-value" { (value.value.to_display_string(Some(1))) }
        }
    }
}
