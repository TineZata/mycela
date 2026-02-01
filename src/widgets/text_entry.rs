use maud::{html, Markup};
use crate::{AppState, config::WidgetConfig};
use crate::pv_monitor::{PvValue, ConnectionStatus};

/// Text entry widget - editable PV value
pub fn render_text_entry(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    tracing::debug!("render_text_entry called for widget: {} with value: {:?}", widget.id, value.is_some());
    
    let (icon_html, alarm_class) = if let Some(v) = value {
        tracing::debug!("Connection status: {:?}, Alarm severity: {}", v.connection_status, v.alarm_severity);
        let class_name = super::alarm_severity_class(v.alarm_severity);
        let icon = super::get_status_icon(&v.connection_status, v.alarm_severity);
        (icon, format!("text-entry {}", class_name))
    } else {
        (Some(super::OFFLINE_SVG), "text-entry alarm-disconnected".to_string())
    };

    let is_integer_type = widget.data_type.as_deref() == Some("integer")  || widget.data_type.as_deref() == Some("int") || widget.data_type.as_deref() == Some("i32");
    
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
    
    let disabled = !matches!(value.map(|v| &v.connection_status), Some(&ConnectionStatus::Connected));
    
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
            
                div class="text-entry-with-icon-container" {
                    @if let Some(icon) = icon_html {
                        img class="text-entry-icon" src=(icon) alt="status";
                    }
                    @if is_string_type {
                        input type="text"
                            class=(alarm_class)
                            name="value"
                            value=(current_value)
                            disabled[disabled]
                            hx-post={"/api/widget/" (widget.id) "/set"}
                            hx-trigger="keyup[key=='Enter']"
                            hx-target="next .status"
                            hx-swap="innerHTML";
                    } @else {
                        input type=(input_type)
                            class=(alarm_class)
                            name="value"
                            value=(current_value)
                            data-original-value=(current_value)
                            step=(format!("{}", step_value))
                            disabled[disabled]
                            hx-post={"/api/widget/" (widget.id) "/set"}
                            hx-trigger="keyup[key=='Enter']"
                            hx-target="next .status"
                            hx-swap="innerHTML"
                            hx-on--before-request="if(isNaN(parseFloat(this.value)) || !isFinite(this.value)) { this.value = this.dataset.originalValue; event.preventDefault(); this.parentElement.nextElementSibling.textContent = 'Invalid number'; return false; } else { this.dataset.originalValue = this.value; this.parentElement.nextElementSibling.textContent = ''; }";
                    }
                    
                @if !units.is_empty() {
                    span class="units-overlay" { (units) }
                }
            }
            
            span class="status" {}
            
            @if let Some(desc) = &widget.description {
                @if !desc.is_empty() {
                    p class="widget-description" { (desc) }
                }
            }
            }
        }
    }
}

/// Render only the inner content (for SSE updates)
pub fn render_text_entry_inner(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    let (icon_html, input_class) = if let Some(v) = value {
        let class_name = super::alarm_severity_class(v.alarm_severity);
        let icon = super::get_status_icon(&v.connection_status, v.alarm_severity);
        (icon, format!("text-entry {}", class_name))
    } else {
        (Some(super::OFFLINE_SVG), "text-entry alarm-disconnected".to_string())
    };

    let is_integer_type = widget.data_type.as_deref() == Some("integer")  || widget.data_type.as_deref() == Some("int") || widget.data_type.as_deref() == Some("i32");
    
    let current_value = value
        .map(|v| {
            let prec = if is_integer_type { 0 } else { v.precision.unwrap_or(2) };
            v.value.to_display_string(Some(prec))
        })
        .unwrap_or_else(|| "--".to_string());
    
    let units = value.and_then(|v| v.units.as_deref()).unwrap_or("");
    let disabled = !matches!(value.map(|v| &v.connection_status), Some(&ConnectionStatus::Connected));
    let step_value = value.and_then(|v| v.min_step).unwrap_or(0.01);
    let input_type = if step_value == 0.0 { "text" } else { "number" };
    let is_string_type = widget.data_type.as_deref() == Some("string");
    let tooltip_text = value.map(|v| super::generate_tooltip(v)).unwrap_or_default();
    
    html! {
        div {
            div class="widget-inner" title=(tooltip_text) {
                label class="widget-label" { (widget.label) }
            
                div class="text-entry-with-icon-container" {
                    @if let Some(icon) = icon_html {
                        img class="text-entry-icon" src=(icon) alt="status";
                    }
                    @if is_string_type {
                        input type="text"
                            class=(input_class)
                            name="value"
                            value=(current_value)
                            disabled[disabled]
                            hx-post={"/api/widget/" (widget.id) "/set"}
                            hx-trigger="keyup[key=='Enter']"
                            hx-target="next .status"
                            hx-swap="innerHTML";
                    } @else {
                        input type=(input_type)
                            class=(input_class)
                            name="value"
                            value=(current_value)
                            data-original-value=(current_value)
                            step=(format!("{}", step_value))
                            disabled[disabled]
                            hx-post={"/api/widget/" (widget.id) "/set"}
                            hx-trigger="keyup[key=='Enter']"
                            hx-target="next .status"
                            hx-swap="innerHTML"
                            hx-on--before-request="if(isNaN(parseFloat(this.value)) || !isFinite(this.value)) { this.value = this.dataset.originalValue; event.preventDefault(); this.parentElement.nextElementSibling.textContent = 'Invalid number'; return false; } else { this.dataset.originalValue = this.value; this.parentElement.nextElementSibling.textContent = ''; }";
                    }
                    
                @if !units.is_empty() {
                    span class="units-overlay" { (units) }
                }
            }
            
            span class="status" {}
            
            @if let Some(desc) = &widget.description {
                @if !desc.is_empty() {
                    p class="widget-description" { (desc) }
                }
            }
            }
        }
    }
}
