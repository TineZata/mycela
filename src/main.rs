mod pv_monitor;
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

use crate::pv_monitor::PvMonitorManager;
use crate::config::ScreenConfig;

/// Application state shared across handlers
#[derive(Clone)]
struct AppState {
    pv_monitor: Arc<PvMonitorManager>,
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

    // Load configuration
    let config = ScreenConfig::load("examples/demo_config.json")
        .expect("Failed to load demo_config.json");
    tracing::info!("✅ Loaded configuration: {} ({} widgets)", config.title, config.widgets.len());

    // Initialize PVXS client
    let pvxs_client = Arc::new(RwLock::new(pvxs_sys::Context::from_env()?));
    tracing::info!("✅ PVXS client initialized successfully");

    // Create PV monitor manager
    let pv_monitor = Arc::new(PvMonitorManager::new(pvxs_client.clone()));

    // Create shared state
    let state = AppState {
        pv_monitor,
        pvxs_client,
        config: Arc::new(config),
    };

    // Build the application router
    let app = Router::new()
        // Main page - directly show demo screen
        .route("/", get(render_demo_screen))
        
        // Screen routes
        .route("/screen/:screen_id", get(render_screen))
        
        // PV API routes
        .route("/api/pv/:name", get(get_pv))
        .route("/api/pv/:name/set", post(put_pv))
        
        // Live update routes (HTMX polling endpoints)
        .route("/poll/widget/:widget_id", get(poll_widget))
        .route("/poll/group/:group_id", get(poll_widget_group))
        
        // Server-Sent Events for real-time monitoring
        .route("/stream/pv/:name", get(stream_pv))
        
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

/// Main index page
async fn index_page() -> Html<String> {
    let markup = maud::html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { "EPICS Motor Control" }
                
                // Self-hosted HTMX (for airgapped production)
                script src="/static/htmx.min.js" {}
                script src="/static/htmx-sse.js" {}
                
                link rel="stylesheet" href="/static/style.css";
            }
            body {
                header class="main-header" {
                    h1 { "🎛️ EPICS Motor Control" }
                    nav {
                        a href="/" { "Home" }
                        a href="/screen/demo" { "Control Interface" }
                    }
                }
                
                main class="container" {
                    div class="welcome-card" {
                        h2 { "Motor Control Demo" }
                        p class="description" { 
                            "EPICS PV monitoring and control interface" 
                        }
                        
                        div style="margin-top: 2rem; text-align: center;" {
                            a href="/screen/demo" 
                              class="pv-button" 
                              style="display: inline-block; padding: 1.5rem 3rem; text-decoration: none;" {
                                "Open Control Interface"
                            }
                        }
                        
                        div style="margin-top: 2rem; padding: 1rem; background: rgba(255,255,255,0.05); border-radius: 8px;" {
                            h3 style="font-size: 1rem; color: #888; margin-bottom: 0.5rem;" { 
                                "Configuration" 
                            }
                            p style="font-size: 0.9rem; color: #aaa;" {
                                "Widgets defined in: "
                                code { "examples/demo_config.json" }
                            }
                        }
                    }
                }
                
                footer {
                    p { "EPICS Web UI • Powered by Rust + HTMX" }
                }
            }
        }
    };
    
    Html(markup.into_string())
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
                
                link rel="stylesheet" href="/static/style.css";
            }
            body {
                header class="main-header" {
                    h1 { "🎛️ " (state.config.title) }
                }
                
                main class="container" {
                    h2 { (state.config.description) }
                    
                    div class="widget-grid" {
                        @for widget in &state.config.widgets {
                            div class="widget" 
                                hx-get={"/poll/widget/" (widget.id)} 
                                hx-trigger="every 1s" {
                                (widgets::render_widget_from_config(widget, &state).await)
                            }
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

/// Get current PV value (JSON API)
async fn get_pv(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    tracing::debug!("GET /api/pv/{}", name);
    
    let value = state.pv_monitor.get_value(&name).await;
    
    Ok(axum::Json(value))
}

/// Put value to a PV
#[derive(serde::Deserialize)]
struct PutForm {
    value: String,
}

async fn put_pv(
    Path(name): Path<String>,
    State(state): State<AppState>,
    Form(form): Form<PutForm>,
) -> Response {
    tracing::info!("PUT /api/pv/{} = {}", name, form.value);
    
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
    let name_clone = name.clone();
    let client_arc = state.pvxs_client.clone();
    
    let result = tokio::task::spawn_blocking(move || {
        let mut client = client_arc.blocking_write();
        client.put_double(&name_clone, value, 5.0)
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
            tracing::error!("Failed to put PV {}: {}", name, e);
            let error_html = maud::html! {
                span class="error" { "Error: " (e) }
            };
            (StatusCode::BAD_REQUEST, Html(error_html.into_string())).into_response()
        }
        Err(e) => {
            tracing::error!("Task error for PV {}: {}", name, e);
            let error_html = maud::html! {
                span class="error" { "Internal error" }
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Html(error_html.into_string())).into_response()
        }
    }
}

/// Poll a single widget for updates (HTMX endpoint)
async fn poll_widget(
    Path(widget_id): Path<String>,
    State(state): State<AppState>,
) -> Html<String> {
    // Look up widget from config
    let widget = state.config.widgets.iter()
        .find(|w| w.id == widget_id);
    
    if let Some(widget) = widget {
        let markup = widgets::render_widget_from_config(widget, &state).await;
        Html(markup.into_string())
    } else {
        let error = maud::html! {
            div class="widget-error" { "Unknown widget: " (widget_id) }
        };
        Html(error.into_string())
    }
}

/// Poll a group of widgets (more efficient)
async fn poll_widget_group(
    Path(group_id): Path<String>,
    State(state): State<AppState>,
) -> Html<String> {
    tracing::debug!("Polling widget group: {}", group_id);
    
    // Load group configuration
    let config_path = format!("examples/{}_config.json", group_id);
    let config = match ScreenConfig::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            let error = maud::html! {
                div class="error" { "Failed to load config: " (e) }
            };
            return Html(error.into_string());
        }
    };
    
    widgets::render_widget_group(&config.widgets, &state).await
}

/// Server-Sent Events stream for real-time PV monitoring
async fn stream_pv(
    Path(name): Path<String>,
    State(_state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    tracing::info!("Starting SSE stream for PV: {}", name);
    
    // Create a stream that updates every second
    // In production, this would use pvxs monitor subscriptions
    let stream = async_stream::stream! {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        let mut counter = 0.0;
        
        loop {
            interval.tick().await;
            
            // Simulate PV update (replace with real pvxs monitor)
            counter += 1.0;
            let value = 50.0 + (counter * 0.1_f64).sin() * 10.0;
            
            let html = format!(
                r#"<span class="pv-value">{:.2}</span>"#,
                value
            );
            
            yield Ok(Event::default().data(html));
        }
    };
    
    Sse::new(stream)
}
