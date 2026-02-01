use maud::{html, Markup};
use crate::config::WidgetConfig;
use crate::pv_monitor::PvValue;

/// Text update widget - read-only PV value display
pub fn render_text_update(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    tracing::debug!("render_text_update called for widget: {} with value: {:?}", widget.id, value.is_some());
    
    let (icon_html, alarm_class) = if let Some(v) = value {
        tracing::debug!("Connection status: {:?}, Alarm severity: {}", v.connection_status, v.alarm_severity);
        let class_name = super::alarm_severity_class(v.alarm_severity);
        let icon = super::get_status_icon(&v.connection_status, v.alarm_severity);
        (icon, format!("text-update {}", class_name))
    } else {
        (Some(super::OFFLINE_SVG), "text-update alarm-disconnected".to_string())
    };
    
    let is_integer_type = widget.data_type.as_deref() == Some("integer") || widget.data_type.as_deref() == Some("int") || widget.data_type.as_deref() == Some("i32");
    
    let current_value = value
        .map(|v| {
            // Determine precision based on data_type or PV precision
            let prec = if is_integer_type {
                0
            } else {
                v.precision.unwrap_or(2)
            };
            v.value.to_display_string(Some(prec))
        })
        .unwrap_or_else(|| "--".to_string());
    
    let units = value
        .and_then(|v| v.units.as_deref())
        .unwrap_or("");
    
    let step_value = value.and_then(|v| v.min_step).unwrap_or(0.01);
    
    let is_string_type = widget.data_type.as_deref() == Some("string");
    let input_type = if is_string_type { "text" } else { "number" };
    
    let tooltip_text = value.map(|v| super::generate_tooltip(v)).unwrap_or_default();
    
    html! {
        div data-widget-id=(widget.id) 
            data-pv=(widget.pv_name)
            hx-ext="sse"
            sse-connect={"/stream/widget/" (widget.id)}
            sse-swap="message"
            hx-swap="innerHTML" {
            
            div class={"widget-inner"} title=(tooltip_text) {
                label class="widget-label" { (widget.label) }
            
                div class="text-update-with-icon-container" {
                    @if let Some(icon) = icon_html {
                        img class="text-update-icon" src=(icon) alt="status";
                    }
                    @if is_string_type {
                        input type="text"
                            class=(alarm_class)
                            name="value"
                            value=(current_value)
                            disabled[true];
                    } @else {
                        input type=(input_type)
                            class=(alarm_class)
                            name="value"
                            value=(current_value)
                            step=(step_value)
                            data-original-value=(current_value)
                            disabled[true];
                    }
                    
                @if !units.is_empty() {
                    span class="units-overlay" { (units) }
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
pub fn render_text_update_inner(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    
    let (icon_html, alarm_class) = if let Some(v) = value {
        let class_name = super::alarm_severity_class(v.alarm_severity);
        let icon = super::get_status_icon(&v.connection_status, v.alarm_severity);
        (icon, format!("text-update {}", class_name))
    } else {
        (Some(super::OFFLINE_SVG), "text-update alarm-disconnected".to_string())
    };
    
    let is_integer_type = widget.data_type.as_deref() == Some("integer") || widget.data_type.as_deref() == Some("int") || widget.data_type.as_deref() == Some("i32");
    
    let current_value = value
        .map(|v| {
            let prec = if is_integer_type {
                0
            } else {
                v.precision.unwrap_or(2)
            };
            v.value.to_display_string(Some(prec))
        })
        .unwrap_or_else(|| "--".to_string());
    
    let units = value
        .and_then(|v| v.units.as_deref())
        .unwrap_or("");
    
    let step_value = value.and_then(|v| v.min_step).unwrap_or(0.01);
    let tooltip_text = value.map(|v| super::generate_tooltip(v)).unwrap_or_default();
    let is_string_type = widget.data_type.as_deref() == Some("string");
    let input_type = if is_string_type { "text" } else { "number" };

    html! {
        div class="widget-inner" title=(tooltip_text) {
            label class="widget-label" { (widget.label) }
        
            div class="text-update-with-icon-container" {
                @if let Some(icon) = icon_html {
                    img class="text-update-icon" src=(icon) alt="status";
                }
                @if is_string_type {
                        input type="text"
                            class=(alarm_class)
                            name="value"
                            value=(current_value)
                            disabled[true];
                } @else {
                    input type=(input_type)
                        class=(alarm_class)
                        name="value"
                        value=(current_value)
                        step=(step_value)
                        data-original-value=(current_value)
                        disabled[true];
                }
                
                @if !units.is_empty() {
                    span class="units-overlay" { (units) }
                }
            }
        }
    }
}
