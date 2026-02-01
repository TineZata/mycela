mod pv_monitor;
mod pv_server;
mod widgets;
mod config;

use axum::{
    Router,
    routing::{get, post},
    extract::{Path, State, Form},
    response::{Html, IntoResponse, Response, sse::{Event, Sse}},
    http::StatusCode,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::{
    services::ServeDir,
    trace::TraceLayer,
    cors::CorsLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::pv_monitor::{PvMonitorManager, NTType};
use crate::pv_server::PvServerManager;
use crate::config::ScreenConfig;

/// Application state shared across handlers
#[derive(Clone)]
struct AppState {
    pv_monitor: Arc<PvMonitorManager>,
    pv_server: Arc<PvServerManager>,
    pvxs_client: Arc<RwLock<pvxs_sys::Context>>,
    config: Arc<ScreenConfig>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ctrl_sys_widgets=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting EPICS Web UI Server");

    // Load configuration - try multiple paths to handle different execution contexts
    let config_paths = [
        "examples/demo_config.json",           // Running from project root
        "../examples/demo_config.json",        // Running from target/debug or target/release
        "../../examples/demo_config.json",     // Running from nested target dirs
    ];
    
    let config = config_paths.iter()
        .find_map(|path| {
            match ScreenConfig::load(path) {
                Ok(cfg) => {
                    tracing::info!("✅ Loaded configuration from: {}", path);
                    Some(cfg)
                }
                Err(e) => {
                    tracing::debug!("Could not load config from {}: {}", path, e);
                    None
                }
            }
        })
        .expect("Failed to load demo_config.json from any expected location. Try running from project root.");
    
    tracing::info!("✅ Loaded configuration: {} ({} widgets)", config.title, config.widgets.len());
    for (idx, widget) in config.widgets.iter().enumerate() {
        tracing::info!("  Widget {}: id={}, type={:?}, label='{}', pv={}", 
            idx, widget.id, widget.widget_type, widget.label, widget.pv_name);
    }

    // Initialize PVXS server if any widgets have server configuration
    let pv_server = Arc::new(PvServerManager::new().expect("Failed to create PV server manager"));
    
    let widgets_with_server: Vec<_> = config.widgets.iter()
        .filter(|w| w.server.is_some())
        .collect();
    
    if !widgets_with_server.is_empty() {
        tracing::info!("Found {} widgets with server configuration", widgets_with_server.len());
        pv_server.start(&config.widgets).expect("Failed to start PVXS server");
    } else {
        tracing::info!("No server PVs configured, running in client-only mode");
    }

    // Initialize PVXS client
    let pvxs_client = Arc::new(RwLock::new(pvxs_sys::Context::from_env()?));
    tracing::info!("✅ PVXS client initialized successfully");

    // Create PV monitor manager
    let pv_monitor = Arc::new(PvMonitorManager::new(pvxs_client.clone()));

    // Create shared state
    let state = AppState {
        pv_monitor,
        pv_server,
        pvxs_client,
        config: Arc::new(config),
    };

    // Build the application router
    let app = Router::new()
        // Main page - directly show demo screen
        .route("/", get(render_demo_screen))
        
        // Screen routes
        .route("/screen/{screen_id}", get(render_screen))
        
        // PV API routes (using widget ID)
        .route("/api/widget/{widget_id}/value", get(get_pv))
        .route("/api/widget/{widget_id}/set", post(put_pv))
        
        // Server control routes
        .route("/api/server/start", post(start_server))
        .route("/api/server/stop", post(stop_server))
        .route("/api/server/status", get(server_status))
        
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
                            class=(if state.pv_server.is_running() { "success" } else { "warning" }) {
                            span { 
                                @if state.pv_server.is_running() {
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
                            (widgets::render_widget_from_config(widget, &state).await)
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
    State(state): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    tracing::info!("Rendering screen: {}", screen_id);
    
    // Load screen configuration
    let config_path = format!("examples/{}_config.json", screen_id);
    let config = ScreenConfig::load(&config_path)
        .map_err(|e| {
            tracing::error!("Failed to load screen config: {}", e);
            StatusCode::NOT_FOUND
        })?;
    
    let markup = widgets::render_screen(&config, &state).await;
    
    Ok(Html(markup.into_string()))
}

/// Get current PV value (JSON API) - using widget ID from config
async fn get_pv(
    Path(widget_id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    tracing::debug!("GET /api/widget/{}/value", widget_id);
    
    // Look up widget in config to get PV name and data_type
    let widget = state.config.widgets.iter()
        .find(|w| w.id == widget_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let pv_name = &widget.pv_name;
    let data_type = &widget.data_type;
    
    tracing::debug!("Widget '{}' -> PV: {}, data_type: {}", widget_id, pv_name, data_type.as_deref().unwrap_or("unknown"));
    
    let value = state.pv_monitor.get_value(pv_name.clone(), data_type).await;
    
    Ok(axum::Json(value))
}

/// Put value to a PV
#[derive(serde::Deserialize)]
struct PutForm {
    value: String,
}

async fn put_pv(
    Path(widget_id): Path<String>,
    State(state): State<AppState>,
    Form(form): Form<PutForm>,
) -> Response {
    tracing::info!("PUT /api/widget/{}/set = {}", widget_id, form.value);
    
    // Look up widget in config to get PV name and data_type
    let widget = match state.config.widgets.iter().find(|w| w.id == widget_id) {
        Some(w) => w,
        None => {
            let error_html = maud::html! {
                span class="error" { "Widget not found: " (widget_id) }
            };
            return (StatusCode::NOT_FOUND, Html(error_html.into_string())).into_response();
        }
    };
    
    let pv_name = widget.pv_name.clone();
    let data_type = &widget.data_type;
    
    tracing::debug!("Widget '{}' -> PV: {}, data_type: {}", widget_id, pv_name, data_type.as_deref().unwrap_or("unknown"));
    
    // Parse the value
    let value: f64 = match form.value.parse() {
        Ok(v) => v,
        Err(e) => {
            let error_html = maud::html! {
                span class="error" { "Invalid number: " (e.to_string()) }
            };
            return Html(error_html.into_string()).into_response();
        }
    };
    
    // Perform the put operation
    let pv_name_for_log = pv_name.clone();
    let client_arc = state.pvxs_client.clone();
    
    let result = tokio::task::spawn_blocking(move || {
        let mut client = client_arc.blocking_write();
        client.put_double(&pv_name, value, 5.0)
            .map_err(|e| e.to_string())
    }).await;
    
    match result {
        Ok(Ok(_)) => {
            // Invalidate cache
            // Monitor will automatically update with new value
            let success_html = maud::html! {
                span class="success" { "✓" }
            };
            Html(success_html.into_string()).into_response()
        }
        Ok(Err(e)) => {
            tracing::error!("Failed to put PV {}: {}", pv_name_for_log, e);
            let error_html = maud::html! {
                span class="error" { "Error: " (e) }
            };
            Html(error_html.into_string()).into_response()
        }
        Err(e) => {
            tracing::error!("Task failed for PV {}: {}", pv_name_for_log, e);
            let error_html = maud::html! {
                span class="error" { "Internal error" }
            };
            Html(error_html.into_string()).into_response()
        }
    }
}

/// Server-Sent Events stream for widget updates driven by PVXS monitors
async fn stream_widget(
    Path(widget_id): Path<String>,
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    tracing::info!("Starting SSE stream for widget ID: {}", widget_id);
    
    // Find the widget config by ID
    let widget_config = state.config.widgets.iter()
        .find(|w| w.id == widget_id)
        .cloned();
    
    if widget_config.is_none() {
        tracing::error!("Widget not found with ID: {}", widget_id);
    }
    
    let pv_name = widget_config.as_ref().map(|w| w.pv_name.clone()).unwrap_or_else(|| {
        tracing::error!("No PV name for widget ID: {}", widget_id);
        String::new()
    });

    let pv_data_type: Option<String> = widget_config.as_ref().and_then(|w| w.data_type.clone());
    
    tracing::info!("SSE stream for widget '{}' monitoring PV: {}", widget_id, pv_name);
    
    let monitor = state.pv_monitor.clone();
    
    let stream = async_stream::stream! {
        // If no valid widget config, return empty stream
        if widget_config.is_none() || pv_name.is_empty() {
            tracing::error!("Cannot start stream for widget '{}' - invalid configuration", widget_id);
            return;
        }
        
        let mut last_value: Option<NTType> = None;
        let mut last_connection: Option<String> = None;
        let mut last_alarm: Option<i32> = None;
        
        // Send initial state immediately
        let pv_value = monitor.get_value(pv_name.clone(), &pv_data_type).await;
        if let Some(widget) = &widget_config {
            let markup = widgets::render_widget_by_type_public(widget, Some(&pv_value));
            let html = markup.into_string();
            last_value = Some(pv_value.value.clone());
            last_connection = Some(format!("{:?}", pv_value.connection_status));
            last_alarm = Some(pv_value.alarm_severity);
            yield Ok(Event::default().data(html));
        }
        
        // Then poll for updates at 10Hz
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));
        
        loop {
            interval.tick().await;
            
            // Get current PV value from monitor cache
            let pv_value = monitor.get_value(pv_name.clone(), &pv_data_type).await;
            
            // Check if anything significant changed
            let current_value = pv_value.value.clone();
            let current_connection = format!("{:?}", pv_value.connection_status);
            let current_alarm = pv_value.alarm_severity;
            
            // Compare values based on type
            let value_changed = match (&last_value, &current_value) {
                (Some(NTType::Double(old)), NTType::Double(new)) => (old - new).abs() > 0.001,
                (Some(NTType::Int32(old)), NTType::Int32(new)) => old != new,
                (Some(NTType::String(old)), NTType::String(new)) => old != new,
                (Some(NTType::Enum { index: old_idx, .. }), NTType::Enum { index: new_idx, .. }) => old_idx != new_idx,
                (None, _) => true,
                (Some(_), _) => true, // Type changed
            };
            let connection_changed = last_connection.as_ref() != Some(&current_connection);
            let alarm_changed = last_alarm != Some(current_alarm);
            
            if value_changed || connection_changed || alarm_changed {
                tracing::debug!(
                    "PV {} changed - value: {} ({}), conn: {} ({}), alarm: {} ({})",
                    pv_name,
                    current_value.to_display_string(None), value_changed,
                    current_connection, connection_changed,
                    current_alarm, alarm_changed
                );
                
                // Generate and send updated widget HTML
                if let Some(widget) = &widget_config {
                    let markup = widgets::render_widget_by_type_public(widget, Some(&pv_value));
                    let html = markup.into_string();
                    
                    last_value = Some(current_value);
                    last_connection = Some(current_connection);
                    last_alarm = Some(current_alarm);
                    
                    yield Ok(Event::default().data(html));
                }
            }
        }
    };
    
    Sse::new(stream)
}

/// Start the PVXS server
async fn start_server(State(state): State<AppState>) -> Response {
    tracing::info!("POST /api/server/start");
    
    // Get the widgets configuration from state
    let widgets = state.config.widgets.clone();
    
    match state.pv_server.start(&widgets) {
        Ok(_) => {
            let success_html = maud::html! {
                div class="success" hx-swap-oob="true" id="server-status" {
                    span { "🟢 Server Running" }
                }
            };
            Html(success_html.into_string()).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to start server: {}", e);
            let error_html = maud::html! {
                div class="error" { "Error: " (e.to_string()) }
            };
            (StatusCode::BAD_REQUEST, Html(error_html.into_string())).into_response()
        }
    }
}

/// Stop the PVXS server
async fn stop_server(State(state): State<AppState>) -> Response {
    tracing::info!("POST /api/server/stop");
    
    match state.pv_server.stop() {
        Ok(_) => {
            let success_html = maud::html! {
                div class="warning" hx-swap-oob="true" id="server-status" {
                    span { "🔴 Server Stopped" }
                }
            };
            Html(success_html.into_string()).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to stop server: {}", e);
            let error_html = maud::html! {
                div class="error" { "Error: " (e.to_string()) }
            };
            (StatusCode::BAD_REQUEST, Html(error_html.into_string())).into_response()
        }
    }
}

/// Get server status
async fn server_status(State(state): State<AppState>) -> Html<String> {
    let is_running = state.pv_server.is_running();
    
    let status_html = maud::html! {
        div id="server-status" class=(if is_running { "success" } else { "warning" }) {
            span { 
                @if is_running {
                    "🟢 Server Running"
                } @else {
                    "🔴 Server Stopped"
                }
            }
        }
    };
    
    Html(status_html.into_string())
}
