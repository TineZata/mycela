use maud::{html, Markup};
use crate::{AppState, config::WidgetConfig};
use crate::pv_monitor::{PvValue, ConnectionStatus};

/// Text entry widget - editable PV value
pub fn render_text_entry(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    tracing::debug!("render_text_entry called for widget: {} with value: {:?}", widget.id, value.is_some());
    
    let (alarm_class, icon_html, input_class) = if let Some(v) = value {
        tracing::debug!("Connection status: {:?}, Alarm severity: {}", v.connection_status, v.alarm_severity);
        let class_name = super::alarm_severity_class(v.alarm_severity);
        let icon = super::get_status_icon(&v.connection_status, v.alarm_severity);
        (class_name, icon, format!("pv-input {}", class_name))
    } else {
        ("alarm-disconnected", Some(super::OFFLINE_SVG), "pv-input alarm-disconnected".to_string())
    };
    
    let current_value = value
        .map(|v| {
            let prec = v.precision.unwrap_or(2) as usize;
            format!("{:.prec$}", v.value, prec = prec)
        })
        .unwrap_or_else(|| "--".to_string());
    
    let units = value
        .and_then(|v| v.units.as_deref())
        .unwrap_or("");
    
    let disabled = !matches!(value.map(|v| &v.connection_status), Some(&ConnectionStatus::Connected));
    
    let step_value = value.and_then(|v| v.min_step).unwrap_or(0.01);
    let input_type = if step_value == 0.0 { "text" } else { "number" };
    let is_string_type = widget.data_type.as_deref() == Some("string");
    
    let tooltip_text = value.map(|v| super::generate_tooltip(v)).unwrap_or_default();
    
    html! {
        div class={"widget text-entry " (alarm_class)} 
            data-widget-id=(widget.id) 
            data-pv=(widget.pv_name)
            title=(tooltip_text) {
            
            label class="widget-label" { (widget.label) }
            
            div class="text-entry-container" {
                div class="input-with-icon" {
                    @if let Some(icon) = icon_html {
                        img class="input-icon" src=(icon) alt="status";
                    }
                    @if is_string_type {
                        input type="text"
                            class=(input_class)
                            name="value"
                            value=(current_value)
                            disabled[disabled]
                            hx-post={"/api/pv/" (widget.pv_name) "/set"}
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
                            hx-post={"/api/pv/" (widget.pv_name) "/set"}
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
            }
            
            @if let Some(desc) = &widget.description {
                @if !desc.is_empty() {
                    p class="widget-description" { (desc) }
                }
            }
        }
    }
}

pub async fn render_text_entry_with_config(widget: &WidgetConfig, state: &AppState) -> Markup {
    let value = state.pv_monitor.get_value(&widget.pv_name).await;
    
    let alarm_class = super::alarm_severity_class(value.alarm_severity);
    let icon_html = super::get_status_icon(&value.connection_status, value.alarm_severity);
    let input_class = format!("pv-input {}", alarm_class);
    
    let disabled = !matches!(value.connection_status, ConnectionStatus::Connected);
    let units = value.units.as_deref().unwrap_or("");
    let step_value = value.min_step.unwrap_or(0.01);
    let is_string_type = widget.data_type.as_deref() == Some("string");
    let input_type = if is_string_type { "text" } else if step_value == 0.0 { "text" } else { "number" };
    let precision = value.precision.unwrap_or(2) as usize;
    let formatted_value = format!("{:.prec$}", value.value, prec = precision);
    
    html! {
        label class="widget-label" { (widget.label) }
        
        div class="text-entry-container" {
            div class="input-with-icon" {
                @if let Some(icon) = icon_html {
                    img class="input-icon" src=(icon) alt="status";
                }
                @if is_string_type {
                    input type="text"
                        class=(input_class)
                        name="value"
                        value=(formatted_value)
                        disabled[disabled]
                        hx-post={"/api/pv/" (widget.pv_name) "/set"}
                        hx-trigger="keyup[key=='Enter']"
                        hx-vals={"js:{value: event.target.value}"};
                } @else {
                    input type=(input_type)
                        class=(input_class)
                        name="value"
                        value=(formatted_value)
                        data-original-value=(formatted_value)
                        step=(format!("{}", step_value))
                        disabled[disabled]
                        hx-post={"/api/pv/" (widget.pv_name) "/set"}
                        hx-trigger="keyup[key=='Enter']"
                        hx-vals={"js:{value: event.target.value}"}
                        hx-on--before-request="if(isNaN(parseFloat(this.value)) || !isFinite(this.value)) { this.value = this.dataset.originalValue; event.preventDefault(); this.parentElement.nextElementSibling.textContent = 'Invalid number'; return false; } else { this.dataset.originalValue = this.value; this.parentElement.nextElementSibling.textContent = ''; }";
                }
                
                @if !units.is_empty() {
                    span class="units-overlay" { (units) }
                }
            }
            
            span class="status" {}
        }
    }
}

pub async fn render_text_entry_simple(pv_name: &str, label: &str, state: &AppState) -> Markup {
    let value = state.pv_monitor.get_value(pv_name).await;
    
    let alarm_class = super::alarm_severity_class(value.alarm_severity);
    let icon_html = super::get_status_icon(&value.connection_status, value.alarm_severity);
    let input_class = format!("pv-input {}", alarm_class);
    
    let disabled = !matches!(value.connection_status, ConnectionStatus::Connected);
    let units = value.units.as_deref().unwrap_or("");
    let step_value = value.min_step.unwrap_or(0.01);
    let input_type = if step_value == 0.0 { "text" } else { "number" };
    let precision = value.precision.unwrap_or(2) as usize;
    let formatted_value = format!("{:.prec$}", value.value, prec = precision);
    
    html! {
        label class="widget-label" { (label) }
        
        div class="text-entry-container" {
            div class="input-with-icon" {
                @if let Some(icon) = icon_html {
                    img class="input-icon" src=(icon) alt="status";
                }
                input type=(input_type)
                    class=(input_class)
                    name="value"
                    value=(formatted_value)
                    data-original-value=(formatted_value)
                    step=(format!("{}", step_value))
                    disabled[disabled]
                    hx-post={"/api/pv/" (pv_name) "/set"}
                    hx-trigger="keyup[key=='Enter']"
                    hx-vals={"js:{value: event.target.value}"}
                    hx-on--before-request="if(isNaN(parseFloat(this.value)) || !isFinite(this.value)) { this.value = this.dataset.originalValue; event.preventDefault(); this.parentElement.nextElementSibling.textContent = 'Invalid number'; return false; } else { this.dataset.originalValue = this.value; this.parentElement.nextElementSibling.textContent = ''; }";
                
                @if !units.is_empty() {
                    span class="units-overlay" { (units) }
                }
            }
            
            span class="status" {}
        }
    }
}
