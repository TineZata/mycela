mod widgets;
mod config;

use axum::{
    Router,
    routing::{get, post},
    extract::{Path, State, Form},
    response::{Html, IntoResponse, Response, sse::{Event, Sse}},
    http::StatusCode,
};
use std::sync::{Arc, Mutex};
use tower_http::{
    services::ServeDir,
    trace::TraceLayer,
    cors::CorsLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::{ScreenConfig, WidgetConfig};

// ─── Application state ───────────────────────────────────────────────────────

#[derive(Clone)]
struct AppState {
    pv_server: Arc<Mutex<Option<pvxs_sys::Server>>>,
    config: Arc<ScreenConfig>,
}

impl AppState {
    fn is_server_running(&self) -> bool {
        self.pv_server.lock().unwrap().is_some()
    }
}

// ─── Server PV setup ─────────────────────────────────────────────────────────

fn setup_server_pvs(server: &pvxs_sys::Server, widgets: &[WidgetConfig]) -> pvxs_sys::Result<()> {
    for widget in widgets {
        if let Some(server_config) = &widget.server {
            let metadata = build_pv_metadata(server_config);
            match widget.data_type.as_deref() {
                Some("double") | Some("float") => {
                    tracing::info!("Creating DOUBLE PV: {}", widget.pv_name);
                    server.create_pv_double(&widget.pv_name, 0.0, metadata)?;
                }
                Some("int32") | Some("int") | Some("integer") => {
                    tracing::info!("Creating INT32 PV: {}", widget.pv_name);
                    server.create_pv_int32(&widget.pv_name, 0, metadata)?;
                }
                Some("string") | None => {
                    tracing::info!("Creating STRING PV: {}", widget.pv_name);
                    server.create_pv_string(&widget.pv_name, "", metadata)?;
                }
                Some(other) => {
                    tracing::warn!("Unknown data_type '{}' for {}, defaulting to STRING", other, widget.pv_name);
                    server.create_pv_string(&widget.pv_name, "", metadata)?;
                }
            }
            tracing::info!("✓ Added PV: {}", widget.pv_name);
        }
    }
    Ok(())
}

fn build_pv_metadata(server_config: &crate::config::ServerConfig) -> pvxs_sys::NTScalarMetadataBuilder {
    let severity = server_config.alarm_serverity.as_ref()
        .map(|s| parse_alarm_severity(s))
        .unwrap_or(pvxs_sys::AlarmSeverity::NoAlarm);
    let status = server_config.alarm_status.as_ref()
        .map(|s| parse_alarm_status(s))
        .unwrap_or(pvxs_sys::AlarmStatus::NoAlarm);

    let mut builder = pvxs_sys::NTScalarMetadataBuilder::new()
        .alarm(severity, status, server_config.alarm_message.as_deref().unwrap_or(""));

    if let Some(metadata) = &server_config.metadata {
        if let Some(display) = &metadata.display {
            builder = builder.display(pvxs_sys::DisplayMetadata {
                limit_low: display.limit_low as i64,
                limit_high: display.limit_high as i64,
                description: display.description.clone(),
                units: display.units.clone(),
                precision: display.precision,
            });
        }
        if let Some(control) = &metadata.control {
            builder = builder.control(pvxs_sys::ControlMetadata {
                limit_low: control.limit_low,
                limit_high: control.limit_high,
                min_step: control.min_step,
            });
        }
        if let Some(alarm) = &metadata.alarm {
            builder = builder.alarm_metadata(pvxs_sys::AlarmMetadata {
                active: true,
                low_alarm_limit: alarm.low_alarm_limit,
                low_warning_limit: alarm.low_warning_limit,
                high_warning_limit: alarm.high_warning_limit,
                high_alarm_limit: alarm.high_alarm_limit,
                low_alarm_severity: parse_alarm_severity(&alarm.low_alarm_severity),
                low_warning_severity: parse_alarm_severity(&alarm.low_warning_severity),
                high_warning_severity: parse_alarm_severity(&alarm.high_warning_severity),
                high_alarm_severity: parse_alarm_severity(&alarm.high_alarm_severity),
                hysteresis: alarm.hysteresis as u8,
            });
        }
    }
    builder
}

fn parse_alarm_severity(severity: &str) -> pvxs_sys::AlarmSeverity {
    match severity.to_uppercase().as_str() {
        "NONE" => pvxs_sys::AlarmSeverity::NoAlarm,
        "MINOR" => pvxs_sys::AlarmSeverity::Minor,
        "MAJOR" => pvxs_sys::AlarmSeverity::Major,
        "INVALID" => pvxs_sys::AlarmSeverity::Invalid,
        _ => {
            tracing::warn!("Unknown alarm severity '{}', using NoAlarm", severity);
            pvxs_sys::AlarmSeverity::NoAlarm
        }
    }
}

fn parse_alarm_status(status: &str) -> pvxs_sys::AlarmStatus {
    match status.to_uppercase().as_str() {
        "NOALARM" | "NONE" => pvxs_sys::AlarmStatus::NoAlarm,
        "DEVICE" => pvxs_sys::AlarmStatus::DeviceStatus,
        "DRIVER" => pvxs_sys::AlarmStatus::DriverStatus,
        "RECORD" => pvxs_sys::AlarmStatus::RecordStatus,
        "DB" => pvxs_sys::AlarmStatus::DbStatus,
        "CONFIG" => pvxs_sys::AlarmStatus::ConfigStatus,
        "CLIENT" => pvxs_sys::AlarmStatus::ClientStatus,
        _ => {
            tracing::warn!("Unknown alarm status '{}', using DeviceStatus", status);
            pvxs_sys::AlarmStatus::DeviceStatus
        }
    }
}

// ─── Per-widget PVXS client and monitor ──────────────────────────────────────

// ─── Per-widget PVXS client and monitor ──────────────────────────────────────

// ─── Entry point ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ctrl_sys_widgets=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting EPICS Web UI Server");

    let config_paths = [
        "examples/demo_config.json",
        "../examples/demo_config.json",
        "../../examples/demo_config.json",
    ];

    let config = config_paths
        .iter()
        .find_map(|path| match ScreenConfig::load(path) {
            Ok(cfg) => {
                tracing::info!("✅ Loaded configuration from: {}", path);
                Some(cfg)
            }
            Err(e) => {
                tracing::debug!("Could not load config from {}: {}", path, e);
                None
            }
        })
        .expect("Failed to load demo_config.json from any expected location. Try running from project root.");

    tracing::info!("✅ Loaded configuration: {} ({} widgets)", config.title, config.widgets.len());
    for (idx, widget) in config.widgets.iter().enumerate() {
        tracing::info!(
            "  Widget {}: id={}, type={:?}, label='{}', pv={}",
            idx, widget.id, widget.widget_type, widget.label, widget.pv_name
        );
    }

    let pv_server = {
        let widgets_with_server: Vec<_> = config.widgets.iter().filter(|w| w.server.is_some()).collect();
        if widgets_with_server.is_empty() {
            tracing::info!("No server PVs configured, running in client-only mode");
            Arc::new(Mutex::new(None))
        } else {
            tracing::info!("Found {} widgets with server configuration", widgets_with_server.len());
            let server = pvxs_sys::Server::start_from_env()?;
            setup_server_pvs(&server, &config.widgets)?;
            tracing::info!("✅ PVXS server started successfully");
            Arc::new(Mutex::new(Some(server)))
        }
    };

    let state = AppState {
        pv_server,
        config: Arc::new(config),
    };

    // Build the application router
    let app = Router::new()
        // Main page - directly show demo screen
        .route("/", get(render_demo_screen))
        
        // Screen routes
        .route("/screen/{screen_id}", get(render_screen))
        
        // Server control routes
        .route("/api/server/start", post(start_server))
        .route("/api/server/stop", post(stop_server))
        .route("/api/server/status", get(server_status))
        
        // Widget write endpoint (form post → PVXS put → HTML feedback)
        .route("/api/widget/{widget_id}/set", post(write_widget))

        // Server-Sent Events for real-time monitoring
        .route("/stream/widget/{name}", get(stream_widget))
        
        // Static files (CSS, JS, images)
        .nest_service("/static", ServeDir::new("static"))
        
        // Add shared state
        .with_state(state)
        
        // Add middleware
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    // Bind to address
    let addr = "127.0.0.1:3000";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    tracing::info!("🚀 Server running at http://{}", addr);
    tracing::info!("📊 Open your browser to see the control interface");

    // Start the server
    axum::serve(listener, app).await?;

    Ok(())
}

/// Render demo screen directly on home page
async fn render_demo_screen(State(state): State<AppState>) -> Html<String> {
    tracing::info!("Rendering demo motor control screen");
    
    let markup = maud::html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { (state.config.title) }
                
                // Self-hosted HTMX
                script src="/static/htmx.min.js" {}
                script src="/static/htmx-sse.js" {}
                script src="/static/tooltip.js" {}
                
                link rel="stylesheet" href="/static/style.css";
            }
            body {
                header class="main-header" {
                    h1 { "🎛️ " (state.config.title) }
                    div class="server-controls" style="display: flex; align-items: center; gap: 1rem;" {
                    div id="server-status" 
                            hx-get="/api/server/status" 
                            hx-trigger="load, every 2s" 
                            class=(if state.is_server_running() { "success" } else { "warning" }) {
                            span { 
                                @if state.is_server_running() {
                                    "🟢 Server Running"
                                } @else {
                                    "🔴 Server Stopped"
                                }
                            }
                        }
                        button class="pv-button" 
                                hx-post="/api/server/start" 
                                hx-target="#server-status"
                                style="padding: 0.5rem 1rem;" {
                            "▶️ Start Server"
                        }
                        button class="pv-button" 
                                hx-post="/api/server/stop" 
                                hx-target="#server-status"
                                style="padding: 0.5rem 1rem; background: #dc3545;" {
                            "⏹️ Stop Server"
                        }
                    }
                }
                
                main class="container" {
                    h2 { (state.config.description) }
                    
                    @let num_widgets = state.config.widgets.len();
                    @let columns = if num_widgets <= 2 { num_widgets } else if num_widgets <= 4 { 2 } else if num_widgets <= 6 { 3 } else { 4 };
                    div class="widget-grid" style=(format!("grid-template-columns: repeat({}, 1fr);", columns)) {
                        @for widget in &state.config.widgets {
                            (widgets::render_widget_from_config(widget))
                        }
                    }
                }
            }
        }
    };
    
    Html(markup.into_string())
}

/// Render a specific screen by ID
async fn render_screen(
    Path(screen_id): Path<String>,
) -> Result<Html<String>, StatusCode> {
    tracing::info!("Rendering screen: {}", screen_id);
    
    // Load screen configuration
    let config_path = format!("examples/{}_config.json", screen_id);
    let config = ScreenConfig::load(&config_path)
        .map_err(|e| {
            tracing::error!("Failed to load screen config: {}", e);
            StatusCode::NOT_FOUND
        })?;
    
    let markup = widgets::render_screen(&config);
    
    Ok(Html(markup.into_string()))
}

// Type alias used so stream_widget can return different concrete stream types
// from a single function (TextEntry stream vs. shared-monitor fallback stream).
#[derive(serde::Deserialize)]
struct PutForm {
    value: String,
}

/// Widget write endpoint — form post → PVXS put → HTML feedback span.
async fn write_widget(
    Path(widget_id): Path<String>,
    State(state): State<AppState>,
    Form(form): Form<PutForm>,
) -> Response {
    let widget = state
        .config
        .widgets
        .iter()
        .find(|w| w.id == widget_id)
        .cloned();

    match widget {
        None => (StatusCode::NOT_FOUND, Html(format!("<span class=\"put-err\">Widget '{}' not found</span>", widget_id))).into_response(),
        Some(w) => {
            let markup = widgets::put_pv(w, form.value).await;
            Html(markup.into_string()).into_response()
        }
    }
}

type SseStream = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>> + Send>>;

/// SSE endpoint — one connection per widget instance.
/// Each widget type manages its own PVXS context and monitor thread.
async fn stream_widget(
    Path(widget_id): Path<String>,
    State(state): State<AppState>,
) -> Sse<SseStream> {
    tracing::info!("SSE stream requested for widget: {}", widget_id);

    let Some(config) = state.config.widgets.iter().find(|w| w.id == widget_id).cloned() else {
        tracing::error!("Widget '{}' not found", widget_id);
        let stream: SseStream = Box::pin(async_stream::stream! {
            yield Ok(Event::default().data("<!-- widget not found -->"));
        });
        return Sse::new(stream);
    };

    use crate::config::WidgetType;
    let stream: SseStream = match config.widget_type {
        WidgetType::TextEntry  => Box::pin(widgets::text_entry::TextEntry::new(config).into_sse_stream()),
        WidgetType::TextUpdate => Box::pin(widgets::text_update::TextUpdate::new(config).into_sse_stream()),
        WidgetType::Gauge      => Box::pin(widgets::gauge::Gauge::new(config).into_sse_stream()),
        WidgetType::Led        => Box::pin(widgets::led::Led::new(config).into_sse_stream()),
        WidgetType::Slider     => Box::pin(widgets::slider::Slider::new(config).into_sse_stream()),
        WidgetType::Button     => Box::pin(widgets::button::Button::new(config).into_sse_stream()),
        WidgetType::Chart      => Box::pin(widgets::chart::Chart::new(config).into_sse_stream()),
    };

    Sse::new(stream)
}

/// Start the PVXS server
async fn start_server(State(state): State<AppState>) -> Response {
    tracing::info!("POST /api/server/start");

    if state.is_server_running() {
        let html = maud::html! { div class="warning" { "Server is already running" } };
        return (StatusCode::BAD_REQUEST, Html(html.into_string())).into_response();
    }

    let config = state.config.clone();
    let result = tokio::task::spawn_blocking(move || {
        let server = pvxs_sys::Server::start_from_env()?;
        setup_server_pvs(&server, &config.widgets)?;
        pvxs_sys::Result::Ok(server)
    })
    .await;

    match result {
        Ok(Ok(server)) => {
            *state.pv_server.lock().unwrap() = Some(server);
            let html = maud::html! {
                div class="success" hx-swap-oob="true" id="server-status" {
                    span { "🟢 Server Running" }
                }
            };
            Html(html.into_string()).into_response()
        }
        Ok(Err(e)) => {
            tracing::error!("Failed to start server: {}", e);
            let html = maud::html! { div class="error" { "Error: " (e.to_string()) } };
            (StatusCode::BAD_REQUEST, Html(html.into_string())).into_response()
        }
        Err(e) => {
            tracing::error!("Server start task panicked: {}", e);
            let html = maud::html! { div class="error" { "Internal error" } };
            (StatusCode::INTERNAL_SERVER_ERROR, Html(html.into_string())).into_response()
        }
    }
}

/// Stop the PVXS server
async fn stop_server(State(state): State<AppState>) -> Response {
    tracing::info!("POST /api/server/stop");

    let server = state.pv_server.lock().unwrap().take();
    match server {
        None => {
            let html = maud::html! { div class="warning" { "Server is not running" } };
            (StatusCode::BAD_REQUEST, Html(html.into_string())).into_response()
        }
        Some(server) => {
            let result = tokio::task::spawn_blocking(move || server.stop_drop()).await;
            match result {
                Ok(Ok(())) => {
                    let html = maud::html! {
                        div class="warning" hx-swap-oob="true" id="server-status" {
                            span { "🔴 Server Stopped" }
                        }
                    };
                    Html(html.into_string()).into_response()
                }
                Ok(Err(e)) => {
                    tracing::error!("Failed to stop server: {}", e);
                    let html = maud::html! { div class="error" { "Error: " (e.to_string()) } };
                    (StatusCode::BAD_REQUEST, Html(html.into_string())).into_response()
                }
                Err(e) => {
                    tracing::error!("Server stop task panicked: {}", e);
                    let html = maud::html! { div class="error" { "Internal error" } };
                    (StatusCode::INTERNAL_SERVER_ERROR, Html(html.into_string())).into_response()
                }
            }
        }
    }
}

/// Get server status
async fn server_status(State(state): State<AppState>) -> Html<String> {
    let is_running = state.is_server_running();

    let status_html = maud::html! {
        div id="server-status" class=(if is_running { "success" } else { "warning" }) {
            span {
                @if is_running { "🟢 Server Running" } @else { "🔴 Server Stopped" }
            }
        }
    };

    Html(status_html.into_string())
}
