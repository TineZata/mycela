use maud::{html, Markup};
use crate::{AppState, config::WidgetConfig};
use crate::pv_monitor::{PvValue, ConnectionStatus};

/// LED indicator widget
pub fn render_led(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    let is_on = value.and_then(|v| v.value.as_f64().map(|val| val > 0.5)).unwrap_or(false);
    let led_state = if is_on { "led-on" } else { "led-off" };
    
    let alarm_class = value
        .map(|v| super::alarm_severity_class(v.alarm_severity))
        .unwrap_or("alarm-disconnected");
    
    let tooltip_text = value.map(|v| super::generate_tooltip(v)).unwrap_or_default();
    
    html! {
        div class={"widget led " (alarm_class)} 
            data-widget-id=(widget.id)
            data-pv=(widget.pv_name)
            title=(tooltip_text) {
            
            label class="widget-label" { (widget.label) }
            
            div class="led-container" {
                div class={"led-indicator " (led_state)} {
                    span class="led-light" {}
                }
                span class="led-status" {
                    @if is_on { "ON" } @else { "OFF" }
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

pub async fn render_led_simple(pv_name: &str, label: &str, state: &AppState) -> Markup {
    let value = state.pv_monitor.get_value(pv_name).await;
    let is_connected = matches!(value.connection_status, ConnectionStatus::Connected);
    let is_on = is_connected && value.value.as_f64().unwrap_or(0.0) > 0.5;
    let led_class = if !is_connected {
        "led-disconnected"
    } else if is_on {
        "led-on"
    } else {
        "led-off"
    };
    
    html! {
        div class="led-widget" {
            label { (label) }
            div class={"led-indicator " (led_class)} {}
            span class="led-status" {
                @if !is_connected {
                    "Not Connected"
                } @else if is_on {
                    "ON"
                } @else {
                    "OFF"
                }
            }
        }
    }
}
