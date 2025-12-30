use maud::{html, Markup};
use crate::config::WidgetConfig;
use crate::pv_monitor::PvValue;

/// Chart widget placeholder - would integrate with charting library
pub fn render_chart(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    let alarm_class = value
        .map(|v| super::alarm_severity_class(v.alarm_severity))
        .unwrap_or("alarm-disconnected");
    
    let tooltip_text = value.map(|v| super::generate_tooltip(v)).unwrap_or_default();
    
    html! {
        div class={"widget chart " (alarm_class)} 
            data-widget-id=(widget.id)
            data-pv=(widget.pv_name)
            title=(tooltip_text) {
            
            label class="widget-label" { (widget.label) }
            
            div class="chart-container"
                hx-ext="sse"
                sse-connect={"/stream/widget/" (widget.id)}
                sse-swap="message"
                hx-target=".chart-data" {
                
                canvas class="chart-canvas" width="400" height="200" {
                    "Chart visualization"
                }
                
                div class="chart-data" style="display: none;" {}
            }
            
            @if let Some(desc) = &widget.description {
                @if !desc.is_empty() {
                    p class="widget-description" { (desc) }
                }
            }
        }
    }
}
