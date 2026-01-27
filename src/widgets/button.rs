use maud::{html, Markup};
use crate::config::WidgetConfig;
use crate::pv_monitor::{PvValue, ConnectionStatus};

/// Button widget - triggers PV write on click
pub fn render_button(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    
    let disabled = !matches!(value.map(|v| &v.connection_status), Some(&ConnectionStatus::Connected));
    
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
                button class="pv-button"
                    disabled[disabled]
                    hx-post={"/api/widget/" (widget.id) "/set"}
                    hx-vals=r#"{"value": "1"}"#
                    hx-target="next .status"
                    hx-swap="innerHTML" {
                    @if let Some(icon) = icon_html {
                        img class="button-icon" src=(icon) alt="status";
                    }
                    (widget.label)
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

/// Render only the inner widget content without the outer SSE wrapper
pub fn render_button_inner(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    let disabled = !matches!(value.map(|v| &v.connection_status), Some(&ConnectionStatus::Connected));
    
    let icon_html = value.and_then(|v| super::get_status_icon(&v.connection_status, v.alarm_severity));
    
    let tooltip_text = value.map(|v| super::generate_tooltip(v)).unwrap_or_default();
    
    html! {
        div class="widget-inner" title=(tooltip_text) {
            button class="pv-button"
                disabled[disabled]
                hx-post={"/api/widget/" (widget.id) "/set"}
                hx-vals=r#"{"value": "1"}"#
                hx-target="next .status"
                hx-swap="innerHTML" {
                @if let Some(icon) = icon_html {
                    img class="button-icon" src=(icon) alt="status";
                }
                (widget.label)
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
