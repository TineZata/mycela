use maud::{html, Markup};
use axum::{
    extract::{Path, State, Form},
    response::{Html, IntoResponse, Response},
    http::StatusCode,
};
use crate::config::{ScreenConfig, WidgetConfig, WidgetType};
use crate::AppState;

#[derive(serde::Deserialize)]
pub struct PutForm {
    pub value: String,
}

/// Widget write endpoint — form post → PVXS put → HTML feedback span.
/// Lives here so widget I/O (reads via SSE, writes via put) is fully owned by the widget layer.
pub async fn write_widget(
    Path(widget_id): Path<String>,
    State(state): State<AppState>,
    Form(form): Form<PutForm>,
) -> Response {
    let widget = state.config.widgets.iter().find(|w| w.id == widget_id).cloned();
    match widget {
        None => (StatusCode::NOT_FOUND, Html(format!("<span class=\"put-err\">Widget '{}' not found</span>", widget_id))).into_response(),
        Some(w) => Html(put_pv(w, form.value).await.into_string()).into_response(),
    }
}

// Base64 encoded SVG icons for different alarm states (shared across all widgets)
pub const OFFLINE_SVG: &str = "data:image/svg+xml;base64,PHN2ZyB2ZXJzaW9uPSIxLjEiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgd2lkdGg9IjI0IiBoZWlnaHQ9IjI0IiB2aWV3Qm94PSIwIDAgMjQgMjQiPjxwYXRoIGZpbGw9IiNmYTAwZmEiIHN0cm9rZT0iI2ZmZiIgc3Ryb2tlLWxpbmVqb2luPSJyb3VuZCIgc3Ryb2tlLWxpbmVjYXA9InJvdW5kIiBzdHJva2UtbWl0ZXJsaW1pdD0iNCIgc3Ryb2tlLXdpZHRoPSIxLjUiIGQ9Ik0yLjc1NyA2LjA5N2MwLTEuODQ1IDEuNDk2LTMuMzQgMy4zNC0zLjM0aDExLjgxOWMxLjg0NSAwIDMuMzQgMS40OTUgMy4zNCAzLjM0djExLjgxOWMwIDEuODQ1LTEuNDk1IDMuMzQtMy4zNCAzLjM0aC0xMS44MTljLTEuODQ1IDAtMy4zNC0xLjQ5NS0zLjM0LTMuMzR2LTExLjgxOXoiPjwvcGF0aD48cGF0aCBmaWxsPSJub25lIiBzdHJva2U9IiNmZmYiIHN0cm9rZS1saW5lam9pbj0icm91bmQiIHN0cm9rZS1saW5lY2FwPSJyb3VuZCIgc3Ryb2tlLW1pdGVybGltaXQ9IjQiIHN0cm9rZS13aWR0aD0iMS41IiBkPSJNMTcuODIgMTQuNDAyYzAuMTE2LTAuMjkzIDAuMTgtMC42MTEgMC4xOC0wLjk0NCAwLTEuMzY3LTEuMDc1LTIuNDktMi40NDgtMi42MTQtMC4yODEtMS42NjEtMS43NjQtMi45MjgtMy41NTItMi45MjgtMC4yNjggMC0wLjUyOSAwLjAyOC0wLjc4IDAuMDgyTTkuMTcyIDkuMjVjLTAuMzY5IDAuNDU0LTAuNjI0IDAuOTk5LTAuNzI1IDEuNTk1LTEuMzczIDAuMTI0LTIuNDQ4IDEuMjQ3LTIuNDQ4IDIuNjE0IDAgMS40NSAxLjIwOSAyLjYyNSAyLjcgMi42MjVoNi42YzAuMjc0IDAgMC41MzgtMC4wMzkgMC43ODctMC4xMTNNNi42IDYuNzVsMTAuOCAxMC41Ij48L3BhdGg+PC9zdmc+";

pub const MAJOR_ALARM_SVG: &str = "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjAiIGhlaWdodD0iMjAiIHZpZXdCb3g9IjAgMCAyMCAyMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48Y2lyY2xlIGN4PSIxMCIgY3k9IjEwIiByPSI4IiBmaWxsPSIjZmYwMDAwIi8+PHRleHQgeD0iMTAiIHk9IjE0IiB0ZXh0LWFuY2hvcj0ibWlkZGxlIiBmaWxsPSJ3aGl0ZSIgZm9udC1zaXplPSIxMiIgZm9udC13ZWlnaHQ9ImJvbGQiIGZvbnQtZmFtaWx5PSJBcmlhbCI+ITwvdGV4dD48L3N2Zz4=";

pub const MINOR_ALARM_SVG: &str = "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjAiIGhlaWdodD0iMjAiIHZpZXdCb3g9IjAgMCAyMCAyMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48cGF0aCBkPSJNMTAgMyBMMTcgMTYgTDMgMTYgWiIgZmlsbD0iI2ZmYTUwMCIvPjx0ZXh0IHg9IjEwIiB5PSIxNCIgdGV4dC1hbmNob3I9Im1pZGRsZSIgZmlsbD0id2hpdGUiIGZvbnQtc2l6ZT0iMTAiIGZvbnQtd2VpZ2h0PSJib2xkIiBmb250LWZhbWlseT0iQXJpYWwiPiE8L3RleHQ+PC9zdmc+";

pub const INVALID_SVG: &str = "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjAiIGhlaWdodD0iMjAiIHZpZXdCb3g9IjAgMCAyMCAyMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48Y2lyY2xlIGN4PSIxMCIgY3k9IjEwIiByPSI4IiBmaWxsPSIjOTk5OTk5Ii8+PHRleHQgeD0iMTAiIHk9IjE0IiB0ZXh0LWFuY2hvcj0ibWlkZGxlIiBmaWxsPSJ3aGl0ZSIgZm9udC1zaXplPSIxMiIgZm9udC13ZWlnaHQ9ImJvbGQiIGZvbnQtZmFtaWx5PSJBcmlhbCI+PzwvdGV4dD48L3N2Zz4=";

// Widget type modules
pub mod text_entry;
pub mod text_update;
pub mod gauge;
pub mod led;
pub mod slider;
pub mod button;
pub mod chart;
pub mod select;

// Re-export widget render functions
pub use text_entry::render_text_entry;
pub use text_update::render_text_update;
pub use gauge::render_gauge;
pub use led::render_led;
pub use slider::render_slider;
pub use button::render_button;
pub use chart::render_chart;
pub use select::render_select;

/// Render widget from config — each widget's outer div contains its own SSE connection.
pub fn render_widget_from_config(widget: &WidgetConfig) -> Markup {
    match widget.widget_type {
        WidgetType::TextEntry  => render_text_entry(widget),
        WidgetType::TextUpdate => render_text_update(widget),
        WidgetType::Gauge      => render_gauge(widget),
        WidgetType::Led        => render_led(widget),
        WidgetType::Slider     => render_slider(widget),
        WidgetType::Button     => render_button(widget),
        WidgetType::Chart      => render_chart(widget),
        WidgetType::Select     => render_select(widget),
    }
}

/// Render a complete screen from configuration
pub fn render_screen(config: &ScreenConfig) -> Markup {
    html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { (config.title) }

                script src="/static/htmx.min.js" {}
                script src="/static/htmx-sse.js" {}
                script src="/static/tooltip.js" {}
                link rel="stylesheet" href="/static/style.css";
            }
            body {
                header class="screen-header" {
                    h1 { (config.title) }
                    p class="description" { (config.description) }
                    a href="/" class="back-link" { "← Back to Home" }
                }

                main class="screen-container" {
                    @let num_widgets = config.widgets.len();
                    @let columns = if num_widgets <= 2 { num_widgets } else if num_widgets <= 4 { 2 } else if num_widgets <= 6 { 3 } else { 4 };
                    div class="widget-grid" style=(format!("grid-template-columns: repeat({}, 1fr);", columns)) {
                        @for widget in &config.widgets {
                            (render_widget_from_config(widget))
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

/// Render a group of widgets
/// Write a value to a PV using PVXS. Returns HTML feedback (success or error).
/// Dispatches to put_double / put_int32 / put_string / put_enum based on config.data_type.
pub async fn put_pv(config: WidgetConfig, value_str: String) -> Markup {
    let pv_name = config.pv_name.clone();
    let data_type = config.data_type.clone();

    let result = tokio::task::spawn_blocking(move || -> pvxs_sys::Result<()> {
        let mut ctx = pvxs_sys::Context::from_env()?;
        match data_type.as_deref() {
            Some("int32") | Some("int") | Some("integer") => {
                let v: i32 = value_str.trim().parse()
                    .map_err(|_| pvxs_sys::PvxsError::new(format!("invalid int32: '{}'", value_str.trim())))?;
                ctx.put_int32(&pv_name, v, 5.0)
            }
            Some("enum") => {
                let v: i16 = value_str.trim().parse()
                    .map_err(|_| pvxs_sys::PvxsError::new(format!("invalid enum index: '{}'", value_str.trim())))?;
                ctx.put_enum(&pv_name, v, 5.0)
            }
            Some("double") | Some("Double") => {
                let v: f64 = value_str.trim().parse()
                    .map_err(|_| pvxs_sys::PvxsError::new(format!("invalid float: '{}'", value_str.trim())))?;
                ctx.put_double(&pv_name, v, 5.0)
            }
            _ => {
                // Default to string
                ctx.put_string(&pv_name, value_str.trim(), 5.0)
            }
        }
    })
    .await;

    match result {
        Ok(Ok(())) => html! { span class="put-ok" { "✓" } },
        Ok(Err(e)) => html! { span class="put-err" { "Error: " (e.to_string()) } },
        Err(e)     => html! { span class="put-err" { "Task error: " (e.to_string()) } },
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

// /// Convert timestamp to human-readable string
// pub fn to_human_time_string(timestamp: i64) -> String {
//     let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0).unwrap_or_default();
//     datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
// }
