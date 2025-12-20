use maud::{html, Markup};
use axum::response::Html;
use crate::{AppState, config::{ScreenConfig, WidgetConfig, WidgetType}};
use crate::pv_monitor::{PvValue, ConnectionStatus};

/// Render a complete screen from configuration
pub async fn render_screen(config: &ScreenConfig, state: &AppState) -> Markup {
    html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { (config.title) }
                
                script src="/static/htmx.min.js" {}
                script src="/static/htmx-sse.js" {}
                link rel="stylesheet" href="/static/style.css";
            }
            body {
                header class="screen-header" {
                    h1 { (config.title) }
                    p class="description" { (config.description) }
                    a href="/" class="back-link" { "← Back to Home" }
                }
                
                main class="screen-container" {
                    // Render the widget grid with auto-polling
                    div class="widget-grid" 
                        hx-get={"/poll/group/" (config.id)}
                        hx-trigger="every 1s"
                        hx-swap="innerHTML" {
                        
                        // Initial render of all widgets
                        @for widget in &config.widgets {
                            (render_widget_static(widget, state).await)
                        }
                    }
                }
                
                footer {
                    p class="screen-footer" {
                        "Screen: " (config.id) " • "
                        span class="widget-count" { (config.widgets.len()) " widgets" }
                    }
                }
            }
        }
    }
}

/// Render a group of widgets (for polling updates)
pub async fn render_widget_group(widgets: &[WidgetConfig], state: &AppState) -> Html<String> {
    let markup = html! {
        @for widget in widgets {
            (render_widget_static(widget, state).await)
        }
    };
    Html(markup.into_string())
}

/// Render a single widget with current data (initial load)
async fn render_widget_static(widget: &WidgetConfig, state: &AppState) -> Markup {
    // Fetch current PV value
    let pv_value = state.pv_monitor.get_value(&widget.pv_name).await;
    
    render_widget_by_type(widget, Some(&pv_value))
}

/// Render widget value update (for HTMX polling)
pub fn render_widget_value(widget_id: &str, pv_name: &str, value: &PvValue) -> Html<String> {
    let markup = html! {
        div class="widget text-entry" data-widget-id=(widget_id) {
            (render_pv_value_inline(pv_name, value))
        }
    };
    Html(markup.into_string())
}

/// Render widget HTML based on type
fn render_widget_by_type(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    match widget.widget_type {
        WidgetType::TextEntry => render_text_entry(widget, value),
        WidgetType::Gauge => render_gauge(widget, value),
        WidgetType::LED => render_led(widget, value),
        WidgetType::Button => render_button(widget, value),
        WidgetType::Slider => render_slider(widget, value),
        WidgetType::Chart => render_chart(widget, value),
    }
}

/// Text entry widget - editable PV value
fn render_text_entry(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    let alarm_class = value
        .map(|v| alarm_severity_class(v.alarm_severity))
        .unwrap_or("alarm-disconnected");
    
    let current_value = value
        .map(|v| format!("{:.2}", v.value))
        .unwrap_or_else(|| "--".to_string());
    
    let units = value
        .and_then(|v| v.units.as_deref())
        .unwrap_or("");
    
    html! {
        div class={"widget text-entry " (alarm_class)} 
            data-widget-id=(widget.id) 
            data-pv=(widget.pv_name) {
            
            label class="widget-label" { (widget.label) }
            
            div class="text-entry-container" {
                input type="number"
                    class="pv-input"
                    name="value"
                    value=(current_value)
                    step="0.01"
                    hx-post={"/api/pv/" (widget.pv_name) "/set"}
                    hx-trigger="change"
                    hx-target="next .status"
                    hx-swap="innerHTML";
                
                span class="status" {}
                
                @if !units.is_empty() {
                    span class="units" { (units) }
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

/// Gauge widget - read-only numeric display with range
fn render_gauge(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    let alarm_class = value
        .map(|v| alarm_severity_class(v.alarm_severity))
        .unwrap_or("alarm-disconnected");
    
    let current_value = value.map(|v| v.value).unwrap_or(0.0);
    let display_value = format!("{:.2}", current_value);
    
    // Use default range for now
    let (min, max) = (0.0, 100.0);
    
    let percentage = ((current_value - min) / (max - min) * 100.0).clamp(0.0, 100.0);
    
    html! {
        div class={"widget gauge " (alarm_class)} 
            data-widget-id=(widget.id)
            data-pv=(widget.pv_name) {
            
            label class="widget-label" { (widget.label) }
            
            div class="gauge-display" {
                div class="gauge-value" { (display_value) }
                div class="gauge-bar" {
                    div class="gauge-fill" style={"width: " (percentage) "%"} {}
                }
                div class="gauge-range" {
                    span class="min" { (format!("{:.1}", min)) }
                    span class="max" { (format!("{:.1}", max)) }
                }
            }
        }
    }
}

/// LED indicator widget
fn render_led(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    let is_on = value.map(|v| v.value > 0.5).unwrap_or(false);
    let led_state = if is_on { "led-on" } else { "led-off" };
    
    let alarm_class = value
        .map(|v| alarm_severity_class(v.alarm_severity))
        .unwrap_or("alarm-disconnected");
    
    html! {
        div class={"widget led " (alarm_class)} 
            data-widget-id=(widget.id)
            data-pv=(widget.pv_name) {
            
            label class="widget-label" { (widget.label) }
            
            div class="led-container" {
                div class={"led-indicator " (led_state)} {
                    span class="led-light" {}
                }
                span class="led-status" {
                    @if is_on { "ON" } @else { "OFF" }
                }
            }
        }
    }
}

/// Button widget - triggers PV write on click
fn render_button(widget: &WidgetConfig, _value: Option<&PvValue>) -> Markup {
    html! {
        div class="widget button-widget" 
            data-widget-id=(widget.id)
            data-pv=(widget.pv_name) {
            
            button class="pv-button"
                hx-post={"/api/pv/" (widget.pv_name) "/set"}
                hx-vals=r#"{"value": "1"}"#
                hx-target="next .status"
                hx-swap="innerHTML" {
                (widget.label)
            }
            
            span class="status" {}
        }
    }
}

/// Slider widget - adjustable value
fn render_slider(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    let current_value = value.map(|v| v.value).unwrap_or(0.0);
    
    let (min, max) = (0.0, 100.0);
    
    let alarm_class = value
        .map(|v| alarm_severity_class(v.alarm_severity))
        .unwrap_or("alarm-disconnected");
    
    html! {
        div class={"widget slider " (alarm_class)} 
            data-widget-id=(widget.id)
            data-pv=(widget.pv_name) {
            
            label class="widget-label" { (widget.label) }
            
            div class="slider-container" {
                input type="range"
                    class="pv-slider"
                    name="value"
                    min=(format!("{}", min))
                    max=(format!("{}", max))
                    step="0.1"
                    value=(format!("{}", current_value))
                    hx-post={"/api/pv/" (widget.pv_name) "/set"}
                    hx-trigger="change"
                    hx-target="next .slider-value";
                
                span class="slider-value" { (format!("{:.2}", current_value)) }
            }
        }
    }
}

/// Chart widget placeholder - would integrate with charting library
fn render_chart(widget: &WidgetConfig, _value: Option<&PvValue>) -> Markup {
    html! {
        div class="widget chart" 
            data-widget-id=(widget.id)
            data-pv=(widget.pv_name) {
            
            label class="widget-label" { (widget.label) }
            
            div class="chart-container"
                hx-ext="sse"
                sse-connect={"/stream/pv/" (widget.pv_name)}
                sse-swap="message"
                hx-target=".chart-data" {
                
                canvas class="chart-canvas" width="400" height="200" {
                    "Chart visualization"
                }
                
                div class="chart-data" style="display: none;" {}
            }
        }
    }
}

/// Helper: render PV value inline (for updates)
fn render_pv_value_inline(_pv_name: &str, value: &PvValue) -> Markup {
    let alarm_class = alarm_severity_class(value.alarm_severity);
    
    html! {
        span class={"pv-value " (alarm_class)} {
            (format!("{:.2}", value.value))
            @if let Some(units) = &value.units {
                " " (units)
            }
        }
    }
}

/// Map alarm severity to CSS class
fn alarm_severity_class(severity: i32) -> &'static str {
    match severity {
        0 => "alarm-none",
        1 => "alarm-minor",
        2 => "alarm-major",
        _ => "alarm-invalid",
    }
}

// Simple widget renderers without config complexity

pub async fn render_text_entry_simple(pv_name: &str, label: &str, state: &AppState) -> Markup {
    let value = state.pv_monitor.get_value(pv_name).await;
    let status_class = match value.connection_status {
        ConnectionStatus::Connected => "status-connected",
        ConnectionStatus::Disconnected => "status-disconnected",
        ConnectionStatus::Timeout => "status-timeout",
        ConnectionStatus::Error(_) => "status-error",
    };
    
    html! {
        div class={"text-entry-widget " (status_class)} {
            label { (label) }
            @if !matches!(value.connection_status, ConnectionStatus::Connected) {
                div class="connection-status" {
                    @match value.connection_status {
                        ConnectionStatus::Disconnected => "⚠️ Disconnected",
                        ConnectionStatus::Timeout => "⏱️ Not Found (Timeout)",
                        ConnectionStatus::Error(ref msg) => (format!("❌ Error: {}", msg)),
                        _ => ""
                    }
                }
            }
            div class="text-entry-row" {
                input type="number" 
                    step="0.01"
                    value=(value.value)
                    disabled[!matches!(value.connection_status, ConnectionStatus::Connected)]
                    hx-post={"/api/pv/" (pv_name) "/set"}
                    hx-trigger="change"
                    hx-vals={"js:{value: event.target.value}"};
                
                span class="readback" {
                    @if matches!(value.connection_status, ConnectionStatus::Connected) {
                        (format!("{:.2}", value.value))
                        @if let Some(units) = value.units {
                            " " (units)
                        }
                    } @else {
                        "---"
                    }
                }
            }
        }
    }
}

pub async fn render_slider_simple(pv_name: &str, label: &str, state: &AppState) -> Markup {
    let value = state.pv_monitor.get_value(pv_name).await;
    let status_class = match value.connection_status {
        ConnectionStatus::Connected => "status-connected",
        _ => "status-disconnected",
    };
    
    html! {
        div class={"slider-widget " (status_class)} {
            label { (label) }
            input type="range" 
                min="-100" max="100" step="1"
                value=(value.value)
                disabled[!matches!(value.connection_status, ConnectionStatus::Connected)]
                hx-post={"/api/pv/" (pv_name) "/set"}
                hx-trigger="change"
                hx-vals={"js:{value: event.target.value}"};
            span class="slider-value" { (format!("{:.1}", value.value)) }
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
                    span class="gauge-value" { (format!("{:.2}", value.value)) }
                    span class="gauge-units" { (units) }
                } @else {
                    span class="gauge-value disconnected" { "---" }
                }
            }
        }
    }
}

pub async fn render_led_simple(pv_name: &str, label: &str, state: &AppState) -> Markup {
    let value = state.pv_monitor.get_value(pv_name).await;
    let is_connected = matches!(value.connection_status, ConnectionStatus::Connected);
    let is_on = is_connected && value.value > 0.5;
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
