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
        div data-widget-id=(widget.id)
            data-pv=(widget.pv_name)
            hx-ext="sse"
            sse-connect={"/stream/widget/" (widget.id)}
            sse-swap="message"
            hx-swap="innerHTML" {

            div class="widget-inner" title=(tooltip_text) {
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
}

/// Render only the inner widget content without the outer SSE wrapper
pub fn render_gauge_inner(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    let alarm_class = value
        .map(|v| super::alarm_severity_class(v.alarm_severity))
        .unwrap_or("alarm-disconnected");
    
    let current_value = value.and_then(|v| v.value.as_f64()).unwrap_or(0.0);
    
    let (min, max) = value
        .and_then(|v| {
            if let (Some(low), Some(high)) = (v.limit_low, v.limit_high) {
                Some((low, high))
            } else {
                None
            }
        })
        .unwrap_or((0.0, 100.0));
    
    let display_value = value.map(|v| v.value.to_display_string(v.precision)).unwrap_or_else(|| "--".to_string());
    
    let percentage = ((current_value - min) / (max - min) * 100.0).clamp(0.0, 100.0);
    
    let units = value
        .and_then(|v| v.units.as_deref())
        .unwrap_or("");
    
    let icon_html = value.and_then(|v| super::get_status_icon(&v.connection_status, v.alarm_severity));
    
    let tooltip_text = value.map(|v| super::generate_tooltip(v)).unwrap_or_default();
    
    html! {
        div class="widget-inner" title=(tooltip_text) {
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

