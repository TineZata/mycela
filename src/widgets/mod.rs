use maud::{html, Markup};
use std::sync::{Arc, Mutex};
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
    let widget = collect_data_widgets(&state.config.widgets)
        .into_iter()
        .find(|w| w.id == widget_id);
    match widget {
        None => (StatusCode::NOT_FOUND, Html(format!("<span class=\"put-err\">Widget '{}' not found</span>", widget_id))).into_response(),
        Some(w) => {
            Html(put_pv(w, form.value, state.write_ctx.clone()).await.into_string()).into_response()
        }
    }
}

// Base64 encoded SVG icons for different alarm states (shared across all widgets)
pub const OFFLINE_SVG: &str = "data:image/svg+xml;base64,PHN2ZyB2ZXJzaW9uPSIxLjEiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgd2lkdGg9IjI0IiBoZWlnaHQ9IjI0IiB2aWV3Qm94PSIwIDAgMjQgMjQiPjxwYXRoIGZpbGw9IiNmYTAwZmEiIHN0cm9rZT0iI2ZmZiIgc3Ryb2tlLWxpbmVqb2luPSJyb3VuZCIgc3Ryb2tlLWxpbmVjYXA9InJvdW5kIiBzdHJva2UtbWl0ZXJsaW1pdD0iNCIgc3Ryb2tlLXdpZHRoPSIxLjUiIGQ9Ik0yLjc1NyA2LjA5N2MwLTEuODQ1IDEuNDk2LTMuMzQgMy4zNC0zLjM0aDExLjgxOWMxLjg0NSAwIDMuMzQgMS40OTUgMy4zNCAzLjM0djExLjgxOWMwIDEuODQ1LTEuNDk1IDMuMzQtMy4zNCAzLjM0aC0xMS44MTljLTEuODQ1IDAtMy4zNC0xLjQ5NS0zLjM0LTMuMzR2LTExLjgxOXoiPjwvcGF0aD48cGF0aCBmaWxsPSJub25lIiBzdHJva2U9IiNmZmYiIHN0cm9rZS1saW5lam9pbj0icm91bmQiIHN0cm9rZS1saW5lY2FwPSJyb3VuZCIgc3Ryb2tlLW1pdGVybGltaXQ9IjQiIHN0cm9rZS13aWR0aD0iMS41IiBkPSJNMTcuODIgMTQuNDAyYzAuMTE2LTAuMjkzIDAuMTgtMC42MTEgMC4xOC0wLjk0NCAwLTEuMzY3LTEuMDc1LTIuNDktMi40NDgtMi42MTQtMC4yODEtMS42NjEtMS43NjQtMi45MjgtMy41NTItMi45MjgtMC4yNjggMC0wLjUyOSAwLjAyOC0wLjc4IDAuMDgyTTkuMTcyIDkuMjVjLTAuMzY5IDAuNDU0LTAuNjI0IDAuOTk5LTAuNzI1IDEuNTk1LTEuMzczIDAuMTI0LTIuNDQ4IDEuMjQ3LTIuNDQ4IDIuNjE0IDAgMS40NSAxLjIwOSAyLjYyNSAyLjcgMi42MjVoNi42YzAuMjc0IDAgMC41MzgtMC4wMzkgMC43ODctMC4xMTNNNi42IDYuNzVsMTAuOCAxMC41Ij48L3BhdGg+PC9zdmc+";

pub const MAJOR_ALARM_SVG: &str = "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjAiIGhlaWdodD0iMjAiIHZpZXdCb3g9IjAgMCAyMCAyMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48Y2lyY2xlIGN4PSIxMCIgY3k9IjEwIiByPSI4IiBmaWxsPSIjZmYwMDAwIi8+PHRleHQgeD0iMTAiIHk9IjE0IiB0ZXh0LWFuY2hvcj0ibWlkZGxlIiBmaWxsPSJ3aGl0ZSIgZm9udC1zaXplPSIxMiIgZm9udC13ZWlnaHQ9ImJvbGQiIGZvbnQtZmFtaWx5PSJBcmlhbCI+ITwvdGV4dD48L3N2Zz4=";

pub const MINOR_ALARM_SVG: &str = "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjAiIGhlaWdodD0iMjAiIHZpZXdCb3g9IjAgMCAyMCAyMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48cGF0aCBkPSJNMTAgMyBMMTcgMTYgTDMgMTYgWiIgZmlsbD0iI2ZmYTUwMCIvPjx0ZXh0IHg9IjEwIiB5PSIxNCIgdGV4dC1hbmNob3I9Im1pZGRsZSIgZmlsbD0id2hpdGUiIGZvbnQtc2l6ZT0iMTAiIGZvbnQtd2VpZ2h0PSJib2xkIiBmb250LWZhbWlseT0iQXJpYWwiPiE8L3RleHQ+PC9zdmc+";

pub const INVALID_SVG: &str = "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjAiIGhlaWdodD0iMjAiIHZpZXdCb3g9IjAgMCAyMCAyMCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48Y2lyY2xlIGN4PSIxMCIgY3k9IjEwIiByPSI4IiBmaWxsPSIjOTk5OTk5Ii8+PHRleHQgeD0iMTAiIHk9IjE0IiB0ZXh0LWFuY2hvcj0ibWlkZGxlIiBmaWxsPSJ3aGl0ZSIgZm9udC1zaXplPSIxMiIgZm9udC13ZWlnaHQ9ImJvbGQiIGZvbnQtZmFtaWx5PSJBcmlhbCI+PzwvdGV4dD48L3N2Zz4=";

pub const INFO_SVG_LIGHT: &str = "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIyNCIgaGVpZ2h0PSIyNCIgdmlld0JveD0iMCAwIDI0IDI0Ij4KICA8Y2lyY2xlIGN4PSIxMiIgY3k9IjEyIiByPSIxMCIgc3Ryb2tlPSJibGFjayIgc3Ryb2tlLXdpZHRoPSIyIiBmaWxsPSJub25lIi8+CiAgPHJlY3QgeD0iMTEiIHk9IjEwIiB3aWR0aD0iMiIgaGVpZ2h0PSI3IiBmaWxsPSJibGFjayIvPgogIDxjaXJjbGUgY3g9IjEyIiBjeT0iNyIgcj0iMSIgZmlsbD0iYmxhY2siLz4KPC9zdmc+";

pub const INFO_SVG_DARK: &str = "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIyNCIgaGVpZ2h0PSIyNCIgdmlld0JveD0iMCAwIDI0IDI0Ij48Y2lyY2xlIGN4PSIxMiIgY3k9IjEyIiByPSIxMCIgc3Ryb2tlPSJ3aGl0ZSIgc3Ryb2tlLXdpZHRoPSIyIiBmaWxsPSJub25lIi8+PHJlY3QgeD0iMTEiIHk9IjEwIiB3aWR0aD0iMiIgaGVpZ2h0PSI3IiBmaWxsPSJ3aGl0ZSIvPjxjaXJjbGUgY3g9IjEyIiBjeT0iNyIgcj0iMSIgZmlsbD0id2hpdGUiLz48L3N2Zz4=";

// Material Design status icons (new — do not replace the alarm icons above)
/// MD check_circle — green, 20 px — server running / PV connected OK
pub const CHECK_CIRCLE_SVG: &str = "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCIgd2lkdGg9IjIwIiBoZWlnaHQ9IjIwIj48cGF0aCBmaWxsPSIjMDBjYzY2IiBkPSJNMTIgMkM2LjQ4IDIgMiA2LjQ4IDIgMTJzNC40OCAxMCAxMCAxMCAxMC00LjQ4IDEwLTEwUzE3LjUyIDIgMTIgMnptLTIgMTVsLTUtNSAxLjQxLTEuNDFMMTAgMTQuMTdsNy41OS03LjU5TDE5IDhsLTkgOXoiLz48L3N2Zz4=";

/// MD cancel — red, 20 px — server stopped / error
pub const CANCEL_SVG: &str = "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCIgd2lkdGg9IjIwIiBoZWlnaHQ9IjIwIj48cGF0aCBmaWxsPSIjZmYzMzMzIiBkPSJNMTIgMkM2LjQ3IDIgMiA2LjQ3IDIgMTJzNC40NyAxMCAxMCAxMCAxMC00LjQ3IDEwLTEwUzE3LjUzIDIgMTIgMnptNSAxMy41OUwxNS41OSAxNyAxMiAxMy40MSA4LjQxIDE3IDcgMTUuNTkgMTAuNTkgMTIgNyA4LjQxIDguNDEgNyAxMiAxMC41OSAxNS41OSA3IDE3IDguNDEgMTMuNDEgMTIgMTcgMTUuNTl6Ii8+PC9zdmc+";

/// MD bolt — white fill, 16 px — button widget action indicator
pub const BOLT_SVG: &str = "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCIgd2lkdGg9IjE2IiBoZWlnaHQ9IjE2Ij48cGF0aCBmaWxsPSJ3aGl0ZSIgZD0iTTcgMnYxMWgzdjlsNy0xMmgtNGw0LTh6Ii8+PC9zdmc+";

// Widget type modules
pub mod text_entry;
pub mod text_update;
pub mod gauge;
pub mod led;
pub mod slider;
pub mod button;
pub mod toggle_button;
pub mod chart;
pub mod select;
pub mod group;

// Re-export widget render functions
pub use text_entry::render_text_entry;
pub use text_update::render_text_update;
pub use gauge::render_gauge;
pub use led::render_led;
pub use slider::render_slider;
pub use button::render_button;
pub use toggle_button::render_toggle_button;
pub use chart::render_chart;
pub use select::render_select;
pub use group::render_group;

/// Recursively collect all data widgets (non-Group) from a widget tree,
/// flattening children of Group containers so they can each get SSE monitors.
pub fn collect_data_widgets(widgets: &[WidgetConfig]) -> Vec<WidgetConfig> {
    let mut result = Vec::new();
    for w in widgets {
        if w.widget_type == WidgetType::Group {
            if let Some(children) = &w.children {
                result.extend(collect_data_widgets(children));
            }
        } else {
            result.push(w.clone());
        }
    }
    result
}

/// Dispatch a widget monitor, tagging each HTML fragment with the widget ID.
/// Used by the multiplexed `/stream/all` SSE endpoint.
pub fn run_widget_monitor(
    config: WidgetConfig,
    widget_id: String,
    tx: tokio::sync::mpsc::UnboundedSender<(String, String)>,
) {
    // Inner channel: the widget monitor sends plain HTML here
    let (inner_tx, mut inner_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    // Forward thread: tags each HTML string with the widget_id
    let wid = widget_id;
    std::thread::spawn(move || {
        while let Some(html) = inner_rx.blocking_recv() {
            if tx.send((wid.clone(), html)).is_err() {
                break;
            }
        }
    });

    let config = std::sync::Arc::new(config);
    match config.widget_type {
        WidgetType::TextEntry    => text_entry::TextEntry::run_monitor(config, inner_tx),
        WidgetType::TextUpdate   => text_update::TextUpdate::run_monitor(config, inner_tx),
        WidgetType::Gauge        => gauge::Gauge::run_monitor(config, inner_tx),
        WidgetType::Led          => led::Led::run_monitor(config, inner_tx),
        WidgetType::Slider       => slider::Slider::run_monitor(config, inner_tx),
        WidgetType::Button       => button::Button::run_monitor(config, inner_tx),
        WidgetType::ToggleButton => toggle_button::ToggleButton::run_monitor(config, inner_tx),
        WidgetType::Chart        => chart::Chart::run_monitor(config, inner_tx),
        WidgetType::Select       => select::Select::run_monitor(config, inner_tx),
        WidgetType::Group        => return, // Groups have no PV — nothing to monitor
    }
}

/// Render widget from config — each widget's outer div contains its own SSE connection.
pub fn render_widget_from_config(widget: &WidgetConfig) -> Markup {
    match widget.widget_type {
        WidgetType::TextEntry  => render_text_entry(widget),
        WidgetType::TextUpdate => render_text_update(widget),
        WidgetType::Gauge      => render_gauge(widget),
        WidgetType::Led        => render_led(widget),
        WidgetType::Slider     => render_slider(widget),
        WidgetType::Button     => render_button(widget),
        WidgetType::ToggleButton => render_toggle_button(widget),
        WidgetType::Chart      => render_chart(widget),
        WidgetType::Select     => render_select(widget),
        WidgetType::Group      => render_group(widget),
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
                script src="/static/tooltip.js" {}
                link rel="stylesheet" href="/static/style.css";
            }
            body {
                header class="screen-header" {
                    h1 { (config.title) }
                    p class="description" { (config.description) }
                    a href="/" class="back-link" { "← Back to Home" }
                }

                main class="screen-container" hx-sse=(format!("connect:/stream/screen/{}", config.id)) {
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
pub async fn put_pv(config: WidgetConfig, value_str: String, write_ctx: Arc<Mutex<pvxs_sys::Context>>) -> Markup {
    let pv_name = config.pv_name.clone();
    let data_type = config.data_type.clone();
    tracing::info!("[{}] put_pv: pv={}, data_type={:?}, value='{}'", config.id, pv_name, data_type, value_str);

    let result = tokio::task::spawn_blocking(move || -> pvxs_sys::Result<()> {
        let mut ctx = write_ctx.lock().unwrap();
        match data_type.as_deref() {
            Some("int32") | Some("int") | Some("integer") | Some("bool") => {
                let v: i32 = value_str.trim().parse()
                    .map_err(|_| pvxs_sys::PvxsError::new(format!("invalid int32: '{}'", value_str.trim())))?;
                ctx.put_int32(&pv_name, v, 5.0)
            }
            Some("enum") => {
                let v: i16 = value_str.trim().parse()
                    .map_err(|_| pvxs_sys::PvxsError::new(format!("invalid enum index: '{}'", value_str.trim())))?;
                ctx.put_enum(&pv_name, v, 5.0)
            }
            Some("double") | Some("float") | Some("f64") | Some("f32") => {
                let v: f64 = value_str.trim().parse()
                    .map_err(|_| pvxs_sys::PvxsError::new(format!("invalid float: '{}'", value_str.trim())))?;
                ctx.put_double(&pv_name, v, 5.0)
            }
            _ => {
                ctx.put_string(&pv_name, value_str.trim(), 5.0)
            }
        }
    })
    .await;

    match result {
        Ok(Ok(())) => {
            tracing::info!("[{}] put_pv OK for {}", config.id, config.pv_name);
            html! { span class="put-ok" { "✓" } }
        }
        Ok(Err(e)) => {
            tracing::error!("[{}] put_pv error for {}: {}", config.id, config.pv_name, e);
            html! { span class="put-err" { "Error: " (e.to_string()) } }
        }
        Err(e) => {
            tracing::error!("[{}] put_pv task error for {}: {}", config.id, config.pv_name, e);
            html! { span class="put-err" { "Task error: " (e.to_string()) } }
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

/// Map alarm status integer to human-readable string (shared across all widgets)
pub(super) fn alarm_status_str(status: i32) -> &'static str {
    match status {
        0 => "No Alarm",
        1 => "Device",
        2 => "Driver",
        3 => "Record",
        4 => "DB",
        5 => "Config",
        6 => "Client",
        _ => "Unknown",
    }
}

/// Build a tooltip string from live PV metadata — shared by all widgets.
pub(super) fn build_tooltip(config: &crate::config::WidgetConfig, raw: &pvxs_sys::Value) -> String {
    let mut t = String::new();

    t.push_str(&format!("PV: {}\n", config.pv_name));

    if let Ok(v) = raw.get_field_string("display.description") { if !v.is_empty() { t.push_str(&v); t.push('\n'); } }
    if let Ok(v) = raw.get_field_string("display.units")       { if !v.is_empty() { t.push_str(&format!("Units: {}\n", v)); } }
    if let Ok(v) = raw.get_field_int32("display.precision")    { t.push_str(&format!("Precision: {}\n", v)); }
    if let Ok(v) = raw.get_field_double("display.limitLow")    { t.push_str(&format!("Display Low: {}\n", v)); }
    if let Ok(v) = raw.get_field_double("display.limitHigh")   { t.push_str(&format!("Display High: {}\n", v)); }
    if let Ok(v) = raw.get_field_double("control.limitLow")    { t.push_str(&format!("Control Low: {}\n", v)); }
    if let Ok(v) = raw.get_field_double("control.limitHigh")   { t.push_str(&format!("Control High: {}\n", v)); }
    if let Ok(v) = raw.get_field_double("control.minStep")     { if v != 0.0 { t.push_str(&format!("Min Step: {}\n", v)); } }
    if let Ok(v) = raw.get_field_double("valueAlarm.lowAlarmLimit")    { t.push_str(&format!("Low Alarm Limit: {}\n", v)); }
    if let Ok(v) = raw.get_field_double("valueAlarm.lowWarningLimit")  { t.push_str(&format!("Low Warning Limit: {}\n", v)); }
    if let Ok(v) = raw.get_field_double("valueAlarm.highWarningLimit") { t.push_str(&format!("High Warning Limit: {}\n", v)); }
    if let Ok(v) = raw.get_field_double("valueAlarm.highAlarmLimit")   { t.push_str(&format!("High Alarm Limit: {}\n", v)); }

    let severity = raw.get_field_int32("alarm.severity").unwrap_or(0);
    let sev_str = match pvxs_sys::AlarmSeverity::from(severity) {
        pvxs_sys::AlarmSeverity::NoAlarm => "No Alarm",
        pvxs_sys::AlarmSeverity::Minor   => "Minor",
        pvxs_sys::AlarmSeverity::Major   => "Major",
        pvxs_sys::AlarmSeverity::Invalid => "Invalid",
        _                                => "Unknown",
    };
    t.push_str(&format!("Alarm Severity: {}\n", sev_str));
    if let Ok(v) = raw.get_field_int32("alarm.status") { t.push_str(&format!("Alarm Status: {}\n", alarm_status_str(v))); }
    if let Ok(v) = raw.get_field_string("alarm.message") { if !v.is_empty() { t.push_str(&format!("Alarm Message: {}\n", v)); } }

    t.trim_end().to_string()
}

/// Build an inline style string from the widget's optional style config (width/height).
/// Returns `None` when no sizing is configured, so maud's `style=[…]` omits the attribute.
pub(crate) fn widget_container_style(config: &crate::config::WidgetConfig) -> Option<String> {
    let mut s = String::new();
    if let Some(style) = &config.style {
        if let Some(w) = &style.width  { s.push_str(&format!("width:{};",  w)); }
        if let Some(h) = &style.height { s.push_str(&format!("height:{};", h)); }
    }
    if s.is_empty() { None } else { Some(s) }
}

/// Render an info button — two icon variants let CSS pick the right one per theme.
pub(super) fn render_info_btn(tooltip: &str) -> maud::Markup {
    html! {
        button class="widget-info-btn" data-tooltip=(tooltip) type="button" {
            img class="info-icon info-icon--dark"  src=(INFO_SVG_DARK)  alt="info";
            img class="info-icon info-icon--light" src=(INFO_SVG_LIGHT) alt="info";
        }
    }
}


// /// Convert timestamp to human-readable string
// pub fn to_human_time_string(timestamp: i64) -> String {
//     let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0).unwrap_or_default();
//     datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{WidgetConfig, WidgetType, WidgetStyle};

    fn make_widget(style: Option<WidgetStyle>) -> WidgetConfig {
        WidgetConfig {
            id: "test1".into(),
            pv_name: "demo:pv".into(),
            widget_type: WidgetType::Gauge,
            label: "Test".into(),
            data_type: None,
            description: None,
            style,
            server: None,
            options: None,
            orientation: None,
            level: None,
            children: None,
        }
    }

    #[test]
    fn style_none_produces_no_attribute() {
        let w = make_widget(None);
        assert!(widget_container_style(&w).is_none());
        let html = render_gauge(&w).into_string();
        // The outer div should not have a style attribute
        // Extract the opening tag (up to the first '>') and check there
        let outer_tag = &html[..html.find('>').unwrap() + 1];
        assert!(!outer_tag.contains("style="),
                "expected no style on outer div, got: {}", outer_tag);
    }

    #[test]
    fn style_width_height_in_html() {
        let w = make_widget(Some(WidgetStyle {
            width: Some("400px".into()),
            height: Some("200px".into()),
            background: None,
        }));
        let css = widget_container_style(&w).unwrap();
        assert!(css.contains("width:400px;"), "CSS must contain width");
        assert!(css.contains("height:200px;"), "CSS must contain height");

        let html = render_gauge(&w).into_string();
        assert!(html.contains("style=\"width:400px;height:200px;\""),
                "rendered HTML must include inline style, got: {}", html);
    }

    #[test]
    fn style_width_only() {
        let w = make_widget(Some(WidgetStyle {
            width: Some("50%".into()),
            height: None,
            background: None,
        }));
        let css = widget_container_style(&w).unwrap();
        assert_eq!(css, "width:50%;");
        assert!(!css.contains("height"));
    }
}
