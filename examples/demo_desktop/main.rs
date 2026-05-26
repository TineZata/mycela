#[path = "../demo_server/epics_simulator.rs"]
mod epics_simulator;
#[path = "../demo_server/modbus_simulator.rs"]
mod modbus_simulator;
mod assets;

use mycela::app::{
    AppState,
    stop_server, server_status, stop_modbus, modbus_status,
};
use mycela::axum::{
    routing::{get, post},
    extract::{Path, State},
    response::{Html, IntoResponse, Response},
    http::{StatusCode, header},
};
use mycela::tower_http::{trace::TraceLayer, cors::CorsLayer};
use mycela::maud::{self};
use mycela::winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};
use mycela::wry::{WebViewBuilder, NewWindowFeatures, NewWindowResponse};
use mycela::pvxs_sys;
use mycela::channel::ChannelContext;
use mycela::config::AppConfig;
use mycela::server_setup::setup_server_pvs;
use mycela::{modbus_client, widgets};
use std::sync::{Arc, Mutex};

// --- Static file handler (embedded assets) -----------------------------------

async fn static_file_handler(Path(path): Path<String>) -> impl IntoResponse {
    match assets::get_asset(&path) {
        Some((bytes, content_type)) => {
            ([(header::CONTENT_TYPE, content_type)], bytes).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// --- API handlers ------------------------------------------------------------

async fn start_server(State(state): State<AppState>) -> Response {
    tracing::info!("POST /api/server/start");
    if state.is_server_running() {
        let html = maud::html! { div class="warning" { "Server is already running" } };
        return (StatusCode::BAD_REQUEST, Html(html.into_string())).into_response();
    }

    let config = state.config.clone();
    let result = tokio::task::spawn_blocking(move || {
        let server = pvxs_sys::Server::start_from_env()?;
        for screen in &config.screens {
            setup_server_pvs(&server, &screen.widgets)?;
        }
        pvxs_sys::Result::Ok(server)
    })
    .await;

    match result {
        Ok(Ok(server)) => {
            for screen in &state.config.screens {
                epics_simulator::start_demo_simulator(server.handle(), &screen.widgets);
            }
            *state.pv_server.lock().unwrap() = Some(server);
            let html = maud::html! {
                div class="success" hx-swap-oob="true" id="server-status" {
                    span { "Server Running" }
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

async fn start_modbus(State(state): State<AppState>) -> Response {
    tracing::info!("POST /api/modbus/start");
    if state.is_modbus_running() {
        let html = maud::html! { div class="warning" { "Modbus TCP simulator is already running" } };
        return (StatusCode::BAD_REQUEST, Html(html.into_string())).into_response();
    }
    let (sim_h, listener_h) = modbus_simulator::start_modbus_simulator(5020);
    *state.modbus_task.lock().unwrap() = Some(vec![sim_h, listener_h]);
    tracing::info!("Modbus TCP demo simulator restarted on port 5020");
    let html = maud::html! {
        div id="modbus-status" class="success" hx-swap-oob="true" {
            span { "Modbus TCP Running" }
        }
    };
    Html(html.into_string()).into_response()
}

// --- Desktop window ----------------------------------------------------------

struct DesktopApp {
    base_url: String,
    windows:  Vec<(Window, wry::WebView)>,
    proxy:    EventLoopProxy<String>,
}

impl DesktopApp {
    fn create_window(&mut self, event_loop: &ActiveEventLoop, url: &str) {
        let window = event_loop
            .create_window(Window::default_attributes().with_title("Mycela"))
            .expect("failed to create window");
        let proxy = self.proxy.clone();
        let webview = WebViewBuilder::new()
            .with_url(url)
            .with_new_window_req_handler(move |u, _features: NewWindowFeatures| {
                let _ = proxy.send_event(u);
                NewWindowResponse::Deny
            })
            .build(&window)
            .expect("failed to create webview");
        self.windows.push((window, webview));
    }
}

impl ApplicationHandler<String> for DesktopApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.windows.is_empty() {
            return;
        }
        let url = self.base_url.clone();
        self.create_window(event_loop, &url);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, url: String) {
        self.create_window(event_loop, &url);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        id: WindowId,
        event: WindowEvent,
    ) {
        if matches!(event, WindowEvent::CloseRequested) {
            self.windows.retain(|(w, _)| w.id() != id);
            if self.windows.is_empty() {
                event_loop.exit();
            }
        }
    }
}

// --- Entry point -------------------------------------------------------------

fn main() {
    let _log_guard = mycela::logging::init_logging(Some(std::path::Path::new("logs")));
    tracing::info!("Starting Mycela Desktop");

    let config: AppConfig = serde_json::from_str(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/demo_app.json")),
    )
    .expect("embedded demo_app.json is invalid");

    // Channel: background server thread sends the bound port to the main thread.
    let (port_tx, port_rx) = std::sync::mpsc::channel::<u16>();

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        runtime.block_on(async move {
            // EPICS / PVXS setup
            let all_widgets: Vec<_> = config.screens.iter()
                .flat_map(|s| widgets::collect_data_widgets(&s.widgets))
                .collect();
            let pv_server = {
                let has_server_pvs = all_widgets.iter()
                    .any(|w| w.epics_pva().and_then(|e| e.server.as_ref()).is_some());
                if !has_server_pvs {
                    tracing::info!("No server PVs configured, running in client-only mode");
                    Arc::new(Mutex::new(None))
                } else {
                    let server = pvxs_sys::Server::start_from_env()
                        .expect("PVXS server start");
                    for screen in &config.screens {
                        setup_server_pvs(&server, &screen.widgets).expect("PVXS setup");
                    }
                    tracing::info!("PVXS server started");
                    for screen in &config.screens {
                        epics_simulator::start_demo_simulator(server.handle(), &screen.widgets);
                    }
                    Arc::new(Mutex::new(Some(server)))
                }
            };

            let epics_ctx = Arc::new(Mutex::new(
                pvxs_sys::Context::from_env().expect("PVXS context"),
            ));

            // Modbus setup
            let (sim_h, listener_h) = modbus_simulator::start_modbus_simulator(5020);
            tracing::info!("Modbus TCP simulator started on port 5020");
            let modbus_pool = modbus_client::ModbusPool::new();
            let channel_ctx = ChannelContext::new(epics_ctx, modbus_pool);

            let state = AppState {
                pv_server,
                config: Arc::new(config),
                channel_ctx,
                modbus_task: Arc::new(Mutex::new(Some(vec![sim_h, listener_h]))),
            };

            // Bind to an OS-assigned port so nothing is hardcoded.
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("TCP bind");
            let port = listener.local_addr().unwrap().port();
            port_tx.send(port).unwrap();
            tracing::info!("Axum server bound on port {}", port);

            let app = state.screen_routes()
                .route("/api/server/start",              post(start_server))
                .route("/api/server/stop",               post(stop_server))
                .route("/api/server/status",             get(server_status))
                .route("/api/modbus/start",              post(start_modbus))
                .route("/api/modbus/stop",               post(stop_modbus))
                .route("/api/modbus/status",             get(modbus_status))
                .route("/static/{*path}",                get(static_file_handler))
                .with_state(state)
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive());

            axum::serve(listener, app).await.expect("axum serve");
        });
    });

    // Block briefly until the server is ready and has sent its port.
    let port = port_rx.recv().expect("server thread exited before sending port");
    let url  = format!("http://127.0.0.1:{}/", port);
    tracing::info!("Desktop window opening {}", url);

    // winit owns the main thread for the duration of the app.
    let event_loop = EventLoop::<String>::with_user_event().build().unwrap();
    let proxy = event_loop.create_proxy();
    let mut app = DesktopApp { base_url: url, windows: Vec::new(), proxy };
    event_loop.run_app(&mut app).unwrap();
}
