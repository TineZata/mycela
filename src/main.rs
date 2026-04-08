mod widgets;
mod config;
mod server_setup;
mod demo_simulator;

use axum::{
    Router,
    routing::{get, post},
    extract::{Path, State},
    response::{Html, IntoResponse, Response, sse::{Event, KeepAlive, Sse}},
    http::StatusCode,
};
use std::sync::{Arc, Mutex};
use tower_http::{
    services::ServeDir,
    trace::TraceLayer,
    cors::CorsLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use maud::{html, Markup};
use crate::config::{ScreenConfig, WidgetConfig, WidgetType};
use server_setup::setup_server_pvs;

// ─── Application state ───────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub pv_server: Arc<Mutex<Option<pvxs_sys::Server>>>,
    pub config: Arc<ScreenConfig>,
    pub write_ctx: Arc<Mutex<pvxs_sys::Context>>,
}

impl AppState {
    fn is_server_running(&self) -> bool {
        self.pv_server.lock().unwrap().is_some()
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ctrl_demo_server=debug,ctrl_sys_widgets=debug,tower_http=debug,axum=trace".into()),
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
    let data_widgets = widgets::collect_data_widgets(&config.widgets);
    for (idx, widget) in data_widgets.iter().enumerate() {
        tracing::info!(
            "  Widget {}: id={}, type={:?}, label='{}', pv={}",
            idx, widget.id, widget.widget_type, widget.label, widget.pv_name
        );
    }

    let pv_server = {
        let widgets_with_server: Vec<_> = data_widgets.iter().filter(|w| w.server.is_some()).collect();
        if widgets_with_server.is_empty() {
            tracing::info!("No server PVs configured, running in client-only mode");
            Arc::new(Mutex::new(None))
        } else {
            tracing::info!("Found {} widgets with server configuration", widgets_with_server.len());
            let server = pvxs_sys::Server::start_from_env()?;
            setup_server_pvs(&server, &config.widgets)?;
            tracing::info!("✅ PVXS server started successfully");

            // Pass a cloneable ServerHandle to the simulator; the Server itself
            // stays owned by AppState so stop_drop() still works cleanly.
            demo_simulator::start_demo_simulator(server.handle(), &config.widgets);

            Arc::new(Mutex::new(Some(server)))
        }
    };

    let write_ctx = Arc::new(Mutex::new(pvxs_sys::Context::from_env()?));

    let state = AppState {
        pv_server,
        config: Arc::new(config),
        write_ctx,
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
        .route("/api/widget/{widget_id}/set", post(widgets::write_widget))

        // Server-Sent Events for real-time monitoring
        .route("/stream/widget/{name}", get(stream_widget))
        .route("/stream/all", get(stream_all_widgets))
        .route("/stream/screen/{screen_id}", get(stream_screen_widgets))
        
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
    tracing::info!("Rendering widget showcase");
    let markup = render_showcase(&state.config, state.is_server_running());
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

type SseStream = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>> + Send>>;

/// SSE endpoint — one connection per widget instance.
/// Each widget type manages its own PVXS context and monitor thread.
async fn stream_widget(
    Path(widget_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    tracing::info!("SSE stream requested for widget: {}", widget_id);

    // Search through flattened data widgets (nested group children included)
    let data_widgets = widgets::collect_data_widgets(&state.config.widgets);
    let Some(config) = data_widgets.into_iter().find(|w| w.id == widget_id) else {
        tracing::error!("Widget '{}' not found", widget_id);
        let stream: SseStream = Box::pin(async_stream::stream! {
            yield Ok(Event::default().data("<!-- widget not found -->"));
        });
        return Sse::new(stream).keep_alive(KeepAlive::default());
    };

    use crate::config::WidgetType;
    let stream: SseStream = match config.widget_type {
        WidgetType::TextEntry  => Box::pin(widgets::text_entry::TextEntry::new(config).into_sse_stream()),
        WidgetType::TextUpdate => Box::pin(widgets::text_update::TextUpdate::new(config).into_sse_stream()),
        WidgetType::Gauge      => Box::pin(widgets::gauge::Gauge::new(config).into_sse_stream()),
        WidgetType::Led        => Box::pin(widgets::led::Led::new(config).into_sse_stream()),
        WidgetType::Slider     => Box::pin(widgets::slider::Slider::new(config).into_sse_stream()),
        WidgetType::Button     => Box::pin(widgets::button::Button::new(config).into_sse_stream()),
        WidgetType::ToggleButton => Box::pin(widgets::toggle_button::ToggleButton::new(config).into_sse_stream()),
        WidgetType::Chart      => Box::pin(widgets::chart::Chart::new(config).into_sse_stream()),
        WidgetType::Select     => Box::pin(widgets::select::Select::new(config).into_sse_stream()),
        WidgetType::Group      => {
            // Groups have no SSE stream
            let stream: SseStream = Box::pin(async_stream::stream! {
                yield Ok(Event::default().data("<!-- group widget has no stream -->"));
            });
            return Sse::new(stream).keep_alive(KeepAlive::default());
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Multiplexed SSE endpoint — a single connection serves ALL widget updates.
///
/// This avoids the HTTP/1.1 per-domain connection limit (typically 6)
/// which would starve widgets beyond the first 6 from receiving SSE events.
/// Each widget update is sent as a named SSE event (event: {widget_id}).
async fn stream_all_widgets(
    State(state): State<AppState>,
) -> impl IntoResponse {
    tracing::info!("Multiplexed SSE stream requested for all widgets");

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(String, String)>();

    // Flatten group children so nested data widgets get monitors
    let data_widgets = widgets::collect_data_widgets(&state.config.widgets);
    for config in data_widgets {
        let tx = tx.clone();
        let widget_id = config.id.clone();

        tokio::task::spawn_blocking(move || {
            widgets::run_widget_monitor(config, widget_id, tx);
        });
    }
    // Drop our copy so the channel closes when all monitors stop
    drop(tx);

    let stream: SseStream = Box::pin(async_stream::stream! {
        // Drop guard: logs when the browser disconnects and axum drops this stream
        struct SseDropGuard;
        impl Drop for SseDropGuard {
            fn drop(&mut self) {
                tracing::warn!("SSE stream DROPPED — browser disconnected or connection lost");
            }
        }
        let _guard = SseDropGuard;

        while let Some((widget_id, html)) = rx.recv().await {
            yield Ok(Event::default().event(widget_id).data(html));
        }
        tracing::info!("SSE stream ended normally (all senders dropped)");
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Screen-specific SSE endpoint — loads the screen config and spawns monitors
/// for its data widgets (including those nested inside Group containers).
async fn stream_screen_widgets(
    Path(screen_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    tracing::info!("SSE stream requested for screen: {}", screen_id);

    let config_path = format!("examples/{}_config.json", screen_id);
    let config = match ScreenConfig::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to load screen config for SSE: {}", e);
            let stream: SseStream = Box::pin(async_stream::stream! {
                yield Ok(Event::default().data("<!-- screen config not found -->"));
            });
            return Sse::new(stream).keep_alive(KeepAlive::default());
        }
    };

    // Set up server PVs for this screen's widgets if we have a running server
    if let Some(server) = state.pv_server.lock().unwrap().as_ref() {
        if let Err(e) = setup_server_pvs(server, &config.widgets) {
            tracing::warn!("Failed to setup server PVs for screen {}: {}", screen_id, e);
        }
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(String, String)>();

    let data_widgets = widgets::collect_data_widgets(&config.widgets);
    for widget_config in data_widgets {
        let tx = tx.clone();
        let widget_id = widget_config.id.clone();

        tokio::task::spawn_blocking(move || {
            widgets::run_widget_monitor(widget_config, widget_id, tx);
        });
    }
    drop(tx);

    let stream: SseStream = Box::pin(async_stream::stream! {
        struct SseDropGuard(String);
        impl Drop for SseDropGuard {
            fn drop(&mut self) {
                tracing::warn!("Screen '{}' SSE stream DROPPED", self.0);
            }
        }
        let _guard = SseDropGuard(screen_id);

        while let Some((widget_id, html)) = rx.recv().await {
            yield Ok(Event::default().event(widget_id).data(html));
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
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
            // Pass a cloneable ServerHandle to the simulator; the Server itself
            // stays owned by AppState so stop_drop() still works cleanly.
            demo_simulator::start_demo_simulator(server.handle(), &state.config.widgets);

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

/// Widget type display name used in the showcase section badges
fn widget_type_name(wt: &WidgetType) -> &'static str {
    match wt {
        WidgetType::TextEntry    => "TextEntry",
        WidgetType::TextUpdate   => "TextUpdate",
        WidgetType::Gauge        => "Gauge",
        WidgetType::Led          => "Led",
        WidgetType::Slider       => "Slider",
        WidgetType::Button       => "Button",
        WidgetType::ToggleButton => "ToggleButton",
        WidgetType::Select       => "Select",
        WidgetType::Chart        => "Chart",
        WidgetType::Group        => "Group",
    }
}

/// Render the showcase home page — each widget type in its own section with dark + light mockup cards.
/// Widget pairing: first occurrence of each type in config → dark card, second → light card.
fn render_showcase(config: &ScreenConfig, server_running: bool) -> Markup {
    // Each entry: (section_key, widget_type, dark_widget, light_widget)
    // Chart widgets use their ID as the key so each gets its own section.
    let mut pairs: Vec<(String, WidgetType, &WidgetConfig, Option<&WidgetConfig>)> = Vec::new();

    // Recursively collect data widget references (skip Groups, recurse into children)
    fn collect_refs<'a>(widgets: &'a [WidgetConfig], out: &mut Vec<&'a WidgetConfig>) {
        for w in widgets {
            if w.widget_type == WidgetType::Group {
                if let Some(children) = &w.children {
                    collect_refs(children, out);
                }
            } else {
                out.push(w);
            }
        }
    }
    let mut data_widgets: Vec<&WidgetConfig> = Vec::new();
    collect_refs(&config.widgets, &mut data_widgets);

    for widget in data_widgets {
        let key = match widget.widget_type {
            WidgetType::Chart => format!("Chart_{}", widget.id),
            _ => format!("{:?}", widget.widget_type),
        };
        if let Some(entry) = pairs.iter_mut().find(|(k, _, _, _)| *k == key) {
            if entry.3.is_none() {
                entry.3 = Some(widget);
            }
        } else {
            pairs.push((key, widget.widget_type.clone(), widget, None));
        }
    }

    html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { "Widget Showcase — " (config.title) }
                script src="/static/htmx.min.js" {}
                script src="/static/tooltip.js" {}
                link rel="stylesheet" href="/static/style.css";
                style {
                    "body { background: linear-gradient(135deg, #1a1a1a 0%, #2d2d2d 100%); }"
                }
            }
            body {
                header class="main-header" {
                    h1 { (config.title) }
                    div class="server-controls" style="display:flex;align-items:center;gap:1rem;margin-top:0.5rem;" {
                        div id="server-status"
                                hx-get="/api/server/status"
                                hx-trigger="load, every 2s" {
                            @if server_running {
                                span class="success" style="display:flex;align-items:center;gap:0.4rem;" {
                                    img src=(widgets::CHECK_CIRCLE_SVG) alt="running" style="width:20px;height:20px;";
                                    "Server Running"
                                }
                            } @else {
                                span class="warning" style="display:flex;align-items:center;gap:0.4rem;color:var(--alarm-minor)" {
                                    img src=(widgets::CANCEL_SVG) alt="stopped" style="width:20px;height:20px;";
                                    "Server Stopped"
                                }
                            }
                        }
                        button class="widget-button"
                                hx-post="/api/server/start"
                                hx-target="#server-status"
                                style="padding:0.4rem 0.9rem;font-size:0.85rem;" { "▶ Start" }
                        button class="widget-button"
                                hx-post="/api/server/stop"
                                hx-target="#server-status"
                                style="padding:0.4rem 0.9rem;font-size:0.85rem;background:#dc3545;" { "⏹ Stop" }
                    }
                }

                main class="showcase-page" hx-sse="connect:/stream/all" {
                    p class="showcase-description" { (config.description) }

                    div class="theme-toggle-bar" {
                        span { "Highlight theme:" }
                        button id="btn-dark"  onclick="highlightTheme('dark')"  { "🌙 Dark" }
                        button id="btn-light" onclick="highlightTheme('light')" { "☀ Light" }
                        button id="btn-both"  onclick="highlightTheme('both')"  class="active" { "Both" }
                    }

                    @for (_key, wtype, dark_w, light_w) in &pairs {
                        section class="widget-section" {
                            div class="section-header" {
                                span class="widget-type-badge" { (widget_type_name(wtype)) }
                                p class="section-description" {
                                    @if let Some(desc) = &dark_w.description {
                                        (desc)
                                    }
                                }
                            }
                            div class={"theme-pair" @if *wtype == WidgetType::Chart { " theme-pair--chart" }} {
                                div class="mockup-card mockup-card--dark" {
                                    div class="mockup-card__titlebar" {
                                        span class="theme-dot" {}
                                        // Charts show their label; others show "Dark Theme"
                                        @if *wtype == WidgetType::Chart {
                                            span { (dark_w.label) }
                                        } @else {
                                            span { "Dark Theme" }
                                        }
                                    }
                                    div class="mockup-card__screen" data-theme="dark" {
                                        (widgets::render_widget_from_config(dark_w))
                                    }
                                }
                                @if *wtype != WidgetType::Chart {
                                    div class="mockup-card mockup-card--light" {
                                        div class="mockup-card__titlebar" {
                                            span class="theme-dot" {}
                                            span { "Light Theme" }
                                        }
                                        div class="mockup-card__screen" data-theme="light" {
                                            @if let Some(lw) = light_w {
                                                (widgets::render_widget_from_config(lw))
                                            } @else {
                                                p style="color:var(--text-secondary);font-size:0.8rem;" {
                                                    "— no light widget configured —"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                script {
                    (maud::PreEscaped(r#"
function highlightTheme(t) {
    document.querySelectorAll('.mockup-card').forEach(el => el.classList.remove('mockup-card--active'));
    document.querySelectorAll('.theme-toggle-bar button').forEach(b => b.classList.remove('active'));
    if (t === 'dark')  { document.querySelectorAll('.mockup-card--dark').forEach(el => el.classList.add('mockup-card--active')); document.getElementById('btn-dark').classList.add('active'); }
    if (t === 'light') { document.querySelectorAll('.mockup-card--light').forEach(el => el.classList.add('mockup-card--active')); document.getElementById('btn-light').classList.add('active'); }
    if (t === 'both')  { document.querySelectorAll('.mockup-card').forEach(el => el.classList.add('mockup-card--active')); document.getElementById('btn-both').classList.add('active'); }
}
                    "#))
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
