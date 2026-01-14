use maud::{html, Markup};
use axum::response::Html;
use crate::{AppState, config::{ScreenConfig, WidgetConfig, WidgetType}};
use crate::pv_monitor::{PvValue, ConnectionStatus, NTType};

// Base64 encoded SVG icons for different alarm states (shared across all widgets)
pub const OFFLINE_SVG: &str = "data:image/svg+xml;base64,PHN2ZyB2ZXJzaW9uPSIxLjEiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgd2lkdGg9IjI0IiBoZWlnaHQ9IjI0IiB2aWV3Qm94PSIwIDAgMjQgMjQiPjxwYXRoIGZpbGw9IiNmYTAwZmEiIHN0cm9rZT0iI2ZmZiIgc3Ryb2tlLWxpbmVqb2luPSJyb3VuZCIgc3Ryb2tlLWxpbmVjYXA9InJvdW5kIiBzdHJva2UtbWl0ZXJsaW1pdD0iNCIgc3Ryb2tlLXdpZHRoPSIxLjUiIGQ9Ik0yLjc1NyA2LjA5N2MwLTEuODQ1IDEuNDk2LTMuMzQgMy4zNC0zLjM0aDExLjgxOWMxLjg0NSAwIDMuMzQgMS40OTUgMy4zNCAzLjM0djExLjgxOWMwIDEuODQ1LTEuNDk1IDMuMzQtMy4zNCAzLjM0aC0xMS44MTljLTEuODQ1IDAtMy4zNC0xLjQ5NS0zLjM0LTMuMzR2LTExLjgxOXoiPjwvcGF0aD48cGF0aCBmaWxsPSJub25lIiBzdHJva2U9IiNmZmYiIHN0cm9rZS1saW5lam9pbj0icm91bmQiIHN0cm9rZS1saW5lY2FwPSJyb3VuZCIgc3Ryb2tlLW1pdGVybGltaXQ9IjQiIHN0cm9rZS13aWR0aD0iMS41IiBkPSJNMTcuODIgMTQuNDAyYzAuMTE2LTAuMjkzIDAuMTgtMC42MTEgMC4xOC0wLjk0NCAwLTEuMzY3LTEuMDc1LTIuNDktMi40NDgtMi42MTQtMC4yODEtMS42NjEtMS43NjQtMi45MjgtMy41NTItMi45MjgtMC4yNjggMC0wLjUyOSAwLjAyOC0wLjc4IDAuMDgyTTkuMTcyIDkuMjVjLTAuMzY5IDAuNDU0LTAuNjI0IDAuOTk5LTAuNzI1IDEuNTk1LTEuMzczIDAuMTI0LTIuNDQ4IDEuMjQ3LTIuNDQ4IDIuNjE0IDAgMS40NSAxLjIwOSAyLjYyNSAyLjcgMi42MjVoNi42YzAuMjc0IDAgMC41MzgtMC4wMzkgMC43ODctMC4xMTNNNi42IDYuNzVsMTAuOCAxMC41Ij48L3BhdGg+PC9zdmc+";

pub const MAJOR_ALARM_SVG: &str = "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjAiIGhlaWdodD0iMjAiIHZpZXdCb3g9IjAgMCAyMCAyMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48Y2lyY2xlIGN4PSIxMCIgY3k9IjEwIiByPSI4IiBmaWxsPSIjZmYwMDAwIi8+PHRleHQgeD0iMTAiIHk9IjE0IiB0ZXh0LWFuY2hvcj0ibWlkZGxlIiBmaWxsPSJ3aGl0ZSIgZm9udC1zaXplPSIxMiIgZm9udC13ZWlnaHQ9ImJvbGQiIGZvbnQtZmFtaWx5PSJBcmlhbCI+ITwvdGV4dD48L3N2Zz4=";

pub const MINOR_ALARM_SVG: &str = "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjAiIGhlaWdodD0iMjAiIHZpZXdCb3g9IjAgMCAyMCAyMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48cGF0aCBkPSJNMTAgMyBMMTcgMTYgTDMgMTYgWiIgZmlsbD0iI2ZmYTUwMCIvPjx0ZXh0IHg9IjEwIiB5PSIxNCIgdGV4dC1hbmNob3I9Im1pZGRsZSIgZmlsbD0id2hpdGUiIGZvbnQtc2l6ZT0iMTAiIGZvbnQtd2VpZ2h0PSJib2xkIiBmb250LWZhbWlseT0iQXJpYWwiPiE8L3RleHQ+PC9zdmc+";

pub const INVALID_SVG: &str = "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjAiIGhlaWdodD0iMjAiIHZpZXdCb3g9IjAgMCAyMCAyMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48Y2lyY2xlIGN4PSIxMCIgY3k9IjEwIiByPSI4IiBmaWxsPSIjOTk5OTk5Ii8+PHRleHQgeD0iMTAiIHk9IjE0IiB0ZXh0LWFuY2hvcj0ibWlkZGxlIiBmaWxsPSJ3aGl0ZSIgZm9udC1zaXplPSIxMiIgZm9udC13ZWlnaHQ9ImJvbGQiIGZvbnQtZmFtaWx5PSJBcmlhbCI+PzwvdGV4dD48L3N2Zz4=";

// Widget type modules
mod text_entry;
mod text_update;
mod gauge;
mod led;
mod slider;
mod button;
mod chart;

// Re-export widget render functions
pub use text_entry::render_text_entry;
pub use text_update::render_text_update;
pub use gauge::render_gauge;
pub use led::render_led;
pub use slider::render_slider;
pub use button::render_button;
pub use chart::render_chart;

/// Render widget from config - dispatches to appropriate widget type
pub async fn render_widget_from_config(widget: &WidgetConfig, state: &AppState) -> Markup {
    // Fetch current PV value
    let pv_value = state.pv_monitor.get_value(widget.pv_name.clone(), &widget.data_type).await;
    
    // Render widget with consistent pattern
    render_widget_by_type(widget, Some(&pv_value))
}

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
                    // Render the widget grid with SSE (Server-Sent Events) updates
                    div class="widget-grid" {
                        // Initial render of all widgets with SSE connections
                        @for widget in &config.widgets {
                            (render_widget_with_sse(widget, state).await)
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
    let pv_value = state.pv_monitor.get_value(widget.pv_name.clone(), &widget.data_type).await;
    
    render_widget_by_type(widget, Some(&pv_value))
}

/// Render a widget with SSE connection for real-time updates
async fn render_widget_with_sse(widget: &WidgetConfig, state: &AppState) -> Markup {
    let pv_value = state.pv_monitor.get_value(widget.pv_name.clone(), &widget.data_type).await;
    
    html! {
        div hx-ext="sse" 
            sse-connect={"/stream/widget/" (widget.id)} 
            sse-swap="message" 
            hx-swap="innerHTML" {
            (render_widget_by_type(widget, Some(&pv_value)))
        }
    }
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
pub fn render_widget_by_type_public(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    render_widget_by_type(widget, value)
}

/// Render widget HTML based on type (internal)
fn render_widget_by_type(widget: &WidgetConfig, value: Option<&PvValue>) -> Markup {
    match widget.widget_type {
        WidgetType::TextEntry => render_text_entry(widget, value),
        WidgetType::TextUpdate => render_text_update(widget, value),
        WidgetType::Gauge => render_gauge(widget, value),
        WidgetType::Led => render_led(widget, value),
        WidgetType::Button => render_button(widget, value),
        WidgetType::Slider => render_slider(widget, value),
        WidgetType::Chart => render_chart(widget, value),
    }
}

/// Helper: render PV value inline (for updates)
fn render_pv_value_inline(_pv_name: &str, value: &PvValue) -> Markup {
    let alarm_class = alarm_severity_class(value.alarm_severity);
    
    html! {
        span class={"pv-value " (alarm_class)} {
            (value.value.to_display_string(value.precision))
            @if let Some(units) = &value.units {
                " " (units)
            }
        }
    }
}

/// Map alarm severity to CSS class
pub fn alarm_severity_class(severity: i32) -> &'static str {
    match severity {
        0 => "alarm-none",
        1 => "alarm-minor",
        2 => "alarm-major",
        _ => "alarm-invalid",
    }
}

/// Get the appropriate icon SVG based on connection status and alarm severity
pub fn get_status_icon(connection_status: &ConnectionStatus, alarm_severity: i32) -> Option<&'static str> {
    match (connection_status, alarm_severity) {
        (ConnectionStatus::Disconnected, _) | (ConnectionStatus::Timeout, _) | (ConnectionStatus::Error(_), _) => 
            Some(OFFLINE_SVG),
        (ConnectionStatus::Connected, 2) => 
            Some(MAJOR_ALARM_SVG),
        (ConnectionStatus::Connected, 1) => 
            Some(MINOR_ALARM_SVG),
        (ConnectionStatus::Connected, 3) => 
            Some(INVALID_SVG),
        _ => None,
    }
}

/// Generate tooltip text with PV metadata
pub fn generate_tooltip(value: &PvValue) -> String {
    let mut tooltip = String::new();
    
    // Connection and alarm info
    tooltip.push_str(&format!("PV: {}\n", value.name));
    // Description
    if let Some(desc) = &value.description {
        tooltip.push_str(&format!("Description: {}\n", desc));
    }
    
    // Value and display info
    tooltip.push_str(&format!("Value: {}\n", value.value.to_display_string(value.precision)));

    // NTType
    match value.value {
        NTType::String(_) => {
            tooltip.push_str(&format!("Type: String\n"));
        }
        NTType::Double(_) => {
            tooltip.push_str(&format!("Type: Double\n"));
        }
        NTType::Int32(_) => {
            tooltip.push_str(&format!("Type: Int32\n"));
        }
        NTType::Enum { .. } => {
            tooltip.push_str(&format!("Type: Enum\n"));
        }
    }

    tooltip.push_str(&format!("Status: {:?}\n", value.connection_status));
    tooltip.push_str(&format!("Alarm Severity: {}\n", value.alarm_severity));
    tooltip.push_str(&format!("Alarm Status: {}\n", value.alarm_status));
    
    if let Some(msg) = &value.alarm_message {
        tooltip.push_str(&format!("Alarm Message: {}\n", msg));
    }
    
    if let Some(units) = &value.units {
        tooltip.push_str(&format!("Units: {}\n", units));
    }
    if let Some(prec) = value.precision {
        tooltip.push_str(&format!("Precision: {}\n", prec));
    }
    
    // Display limits
    if let Some(low) = value.limit_low {
        tooltip.push_str(&format!("Display Low: {}\n", low));
    }
    if let Some(high) = value.limit_high {
        tooltip.push_str(&format!("Display High: {}\n", high));
    }
    
    // Control limits
    if let Some(low) = value.control_low {
        tooltip.push_str(&format!("Control Low: {}\n", low));
    }
    if let Some(high) = value.control_high {
        tooltip.push_str(&format!("Control High: {}\n", high));
    }
    if let Some(step) = value.min_step {
        tooltip.push_str(&format!("Min Step: {}\n", step));
    }
    
    // Alarm limits
    if let Some(lal) = value.low_alarm_limit {
        tooltip.push_str(&format!("Low Alarm Limit: {}\n", lal));
    }
    if let Some(lwl) = value.low_warning_limit {
        tooltip.push_str(&format!("Low Warning Limit: {}\n", lwl));
    }
    if let Some(hwl) = value.high_warning_limit {
        tooltip.push_str(&format!("High Warning Limit: {}\n", hwl));
    }
    if let Some(hal) = value.high_alarm_limit {
        tooltip.push_str(&format!("High Alarm Limit: {}\n", hal));
    }
    
    // Timestamp
    tooltip.push_str(&format!("Timestamp: {}\n", to_human_time_string(value.timestamp)));
    
    tooltip.trim_end().to_string()
}

/// Convert timestamp to human-readable string
pub fn to_human_time_string(timestamp: i64) -> String {
    // Timestamp is already in Unix epoch format (seconds since 1970-01-01)
    let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0).unwrap_or_default();
    datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}
