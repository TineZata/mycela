use maud::{html, Markup};
use crate::{AppState, config::WidgetConfig};
use crate::pv_monitor::{PvValue, ConnectionStatus};

/// Gauge widget - read-only numeric display with range
pub fn render_gauge(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    let alarm_class = value
        .map(|v| super::alarm_severity_class(v.alarm_severity))
        .unwrap_or("alarm-disconnected");
    
    let current_value = value.and_then(|v| v.value.as_f64()).unwrap_or(0.0);
    
    // Extract display range from PV metadata or use defaults
    let (min, max) = value
        .and_then(|v| {
            if let (Some(low), Some(high)) = (v.limit_low, v.limit_high) {
                Some((low, high))
            } else {
                None
            }
        })
        .unwrap_or((0.0, 100.0));
    
    // Use NTType display method for formatting
    let display_value = value.map(|v| v.value.to_display_string(v.precision)).unwrap_or_else(|| "--".to_string());
    
    let percentage = ((current_value - min) / (max - min) * 100.0).clamp(0.0, 100.0);
    
    let units = value
        .and_then(|v| v.units.as_deref())
        .unwrap_or("");
    
    let icon_html = value.and_then(|v| super::get_status_icon(&v.connection_status, v.alarm_severity));
    
    let tooltip_text = value.map(|v| super::generate_tooltip(v)).unwrap_or_default();
    
    html! {
        div class={"widget gauge " (alarm_class)} 
            data-widget-id=(widget.id)
            data-pv=(widget.pv_name)
            title=(tooltip_text) {
            
            label class="widget-label" { 
                (widget.label)
                @if let Some(icon) = icon_html {
                    img class="widget-status-icon" src=(icon) alt="status";
                }
            }
            
            div class="gauge-display" {
                div class="gauge-value" { 
                    (display_value) 
                    @if !units.is_empty() {
                        " " (units)
                    }
                }
                div class="gauge-bar" {
                    div class="gauge-fill" style={"width: " (percentage) "%"} {}
                }
                div class="gauge-range" {
                    span class="min" { (format!("{:.1}", min)) }
                    span class="max" { (format!("{:.1}", max)) }
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

pub async fn render_gauge_simple(pv_name: &str, label: &str, units: &str, state: &AppState) -> Markup {
    let value = state.pv_monitor.get_value(pv_name).await;
    let status_class = match value.connection_status {
        ConnectionStatus::Connected => "status-connected",
        _ => "status-disconnected",
    };
    
    html! {
        div class={"gauge-widget " (status_class)} {
            label { (label) }
            div class="gauge-display" {
                @if matches!(value.connection_status, ConnectionStatus::Connected) {
                    span class="gauge-value" { (value.value.to_display_string(Some(2))) }
                    span class="gauge-units" { (units) }
                } @else {
                    span class="gauge-value disconnected" { "---" }
                }
            }
        }
    }
}
