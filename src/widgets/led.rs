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
        div data-widget-id=(widget.id)
            data-pv=(widget.pv_name)
            hx-ext="sse"
            sse-connect={"/stream/widget/" (widget.id)}
            sse-swap="message"
            hx-swap="innerHTML" {

            div class="widget-inner" title=(tooltip_text) {
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
}

/// Render only the inner widget content without the outer SSE wrapper
pub fn render_led_inner(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    let is_on = value.and_then(|v| v.value.as_f64().map(|val| val > 0.5)).unwrap_or(false);
    let led_state = if is_on { "led-on" } else { "led-off" };
    
    let alarm_class = value
        .map(|v| super::alarm_severity_class(v.alarm_severity))
        .unwrap_or("alarm-disconnected");
    
    let tooltip_text = value.map(|v| super::generate_tooltip(v)).unwrap_or_default();
    
    html! {
        div class="widget-inner" title=(tooltip_text) {
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
