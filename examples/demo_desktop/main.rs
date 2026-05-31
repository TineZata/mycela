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
use mycela::desktop_transport::{DESKTOP_TRANSPORT_ENV, DesktopTransport};
use mycela::ipc::{IpcRequest, IpcResponse};
use mycela::ipc_dispatch;
use mycela::protocol_control::{self, ProtocolControlError};
use mycela::server_setup::setup_server_pvs;
use mycela::{modbus_client, widgets};
use std::sync::{Arc, Mutex, mpsc};

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
    match protocol_control::start_epics_runtime(&state).await {
        Ok(()) => {
            let html = maud::html! {
                div class="success" hx-swap-oob="true" id="server-status" {
                    span { "Server Running" }
                }
            };
            Html(html.into_string()).into_response()
        }
        Err(ProtocolControlError::AlreadyRunning(_)) => {
            let html = maud::html! { div class="warning" { "Server is already running" } };
            (StatusCode::BAD_REQUEST, Html(html.into_string())).into_response()
        }
        Err(ProtocolControlError::Operation(e)) => {
            tracing::error!("Failed to start server: {}", e);
            let html = maud::html! { div class="error" { "Error: " (e.to_string()) } };
            (StatusCode::BAD_REQUEST, Html(html.into_string())).into_response()
        }
        Err(ProtocolControlError::Internal(e)) => {
            tracing::error!("Server start task panicked: {}", e);
            let html = maud::html! { div class="error" { "Internal error" } };
            (StatusCode::INTERNAL_SERVER_ERROR, Html(html.into_string())).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to start server: {}", e);
            let html = maud::html! { div class="error" { "Internal error" } };
            (StatusCode::INTERNAL_SERVER_ERROR, Html(html.into_string())).into_response()
        }
    }
}

async fn start_modbus(State(state): State<AppState>) -> Response {
    tracing::info!("POST /api/modbus/start");
    match protocol_control::start_modbus_runtime(&state) {
        Ok(()) => {
            tracing::info!("Modbus TCP demo simulator restarted on port 5020");
            let html = maud::html! {
                div id="modbus-status" class="success" hx-swap-oob="true" {
                    span { "Modbus TCP Running" }
                }
            };
            Html(html.into_string()).into_response()
        }
        Err(ProtocolControlError::AlreadyRunning(_)) => {
            let html = maud::html! { div class="warning" { "Modbus TCP simulator is already running" } };
            (StatusCode::BAD_REQUEST, Html(html.into_string())).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to start Modbus simulator: {}", e);
            let html = maud::html! { div class="error" { "Internal error" } };
            (StatusCode::INTERNAL_SERVER_ERROR, Html(html.into_string())).into_response()
        }
    }
}

// --- Desktop window ----------------------------------------------------------

enum DesktopUserEvent {
    OpenWindow(String),
    IpcMessage(String),
}

struct BackendRequest {
    request: IpcRequest,
    response_tx: mpsc::Sender<IpcResponse>,
}

struct DesktopApp {
    transport: DesktopTransport,
    base_url: Option<String>,
    windows:  Vec<(Window, wry::WebView)>,
    proxy:    EventLoopProxy<DesktopUserEvent>,
    backend_tx: Option<mpsc::Sender<BackendRequest>>,
}

impl DesktopApp {
    fn create_window(&mut self, event_loop: &ActiveEventLoop, url: Option<&str>) {
        let window = event_loop
            .create_window(Window::default_attributes().with_title("Mycela"))
            .expect("failed to create window");
        let proxy = self.proxy.clone();
        let webview = match self.transport {
            DesktopTransport::Loopback => WebViewBuilder::new()
                .with_url(url.expect("loopback window requires URL"))
                .with_new_window_req_handler(move |u, _features: NewWindowFeatures| {
                    let _ = proxy.send_event(DesktopUserEvent::OpenWindow(u));
                    NewWindowResponse::Deny
                })
                .build(&window)
                .expect("failed to create webview"),
            DesktopTransport::Ipc => WebViewBuilder::new()
                .with_html(url.expect("ipc window requires HTML shell"))
                .with_ipc_handler(move |request| {
                    let _ = proxy.send_event(DesktopUserEvent::IpcMessage(request.body().clone()));
                })
                .with_new_window_req_handler(|u, _features: NewWindowFeatures| {
                    tracing::warn!("Blocked IPC-mode window.open request to {}", u);
                    NewWindowResponse::Deny
                })
                .build(&window)
                .expect("failed to create webview"),
        };
        self.windows.push((window, webview));
    }

    fn handle_ipc_message(&mut self, payload: String) {
        let request = match serde_json::from_str::<IpcRequest>(&payload) {
            Ok(request) => request,
            Err(error) => {
                tracing::error!("Failed to parse IPC request: {}", error);
                return;
            }
        };

        let Some(backend_tx) = &self.backend_tx else {
            tracing::warn!("Dropped IPC request because no backend is configured");
            return;
        };

        let (response_tx, response_rx) = mpsc::channel();
        if let Err(error) = backend_tx.send(BackendRequest { request, response_tx }) {
            tracing::error!("Failed to send IPC request to backend: {}", error);
            return;
        }

        let response = match response_rx.recv() {
            Ok(response) => response,
            Err(error) => {
                tracing::error!("Failed to receive IPC response from backend: {}", error);
                return;
            }
        };

        let response_json = match serde_json::to_string(&response) {
            Ok(json) => json,
            Err(error) => {
                tracing::error!("Failed to serialize IPC response: {}", error);
                return;
            }
        };
        let script = format!("window.__MYCELA_IPC_DELIVER({});", response_json);

        for (_, webview) in &self.windows {
            if let Err(error) = webview.evaluate_script(&script) {
                tracing::error!("Failed to deliver IPC response to webview: {}", error);
            }
        }
    }
}

impl ApplicationHandler<DesktopUserEvent> for DesktopApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.windows.is_empty() {
            return;
        }
        let url = self.base_url.clone();
        self.create_window(event_loop, url.as_deref());
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: DesktopUserEvent) {
        match event {
            DesktopUserEvent::OpenWindow(url) => {
                if self.transport == DesktopTransport::Loopback {
                    self.create_window(event_loop, Some(&url));
                } else {
                    tracing::warn!("Ignored new window request in IPC mode: {}", url);
                }
            }
            DesktopUserEvent::IpcMessage(payload) => self.handle_ipc_message(payload),
        }
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

fn build_ipc_shell_html(session_token: &str) -> String {
    let token_json = serde_json::to_string(session_token)
        .expect("session token should serialize to JSON string");
    r#"<!doctype html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Mycela Desktop IPC</title>
    <style>
        :root {
            color-scheme: light;
            --bg: #f3efe7;
            --panel: #fffaf2;
            --ink: #1c1a17;
            --accent: #b3541e;
            --accent-soft: #f4d6bf;
            --border: #d8c6b8;
            --mono: "IBM Plex Mono", Consolas, monospace;
            --sans: "Inter", Segoe UI, sans-serif;
        }
        * { box-sizing: border-box; }
        body {
            margin: 0;
            font-family: var(--sans);
            color: var(--ink);
            background: radial-gradient(circle at top left, #fff8ef, var(--bg) 60%);
            min-height: 100vh;
            padding: 24px;
        }
        main {
            max-width: 900px;
            margin: 0 auto;
            background: var(--panel);
            border: 1px solid var(--border);
            border-radius: 20px;
            padding: 24px;
            box-shadow: 0 18px 50px rgba(60, 38, 18, 0.08);
        }
        h1 { margin: 0 0 8px; font-size: 2rem; }
        p { margin: 0 0 16px; line-height: 1.5; }
        .actions {
            display: flex;
            flex-wrap: wrap;
            gap: 12px;
            margin: 20px 0;
        }
        button {
            border: 0;
            border-radius: 999px;
            padding: 10px 16px;
            background: var(--accent);
            color: white;
            font: inherit;
            cursor: pointer;
        }
        button.secondary {
            background: var(--accent-soft);
            color: var(--ink);
        }
        .grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
            gap: 12px;
            margin: 20px 0;
        }
        .card {
            border: 1px solid var(--border);
            border-radius: 14px;
            padding: 16px;
            background: white;
        }
        .card h2 {
            font-size: 0.95rem;
            margin: 0 0 6px;
            text-transform: uppercase;
            letter-spacing: 0.08em;
        }
        .status {
            font-size: 1.8rem;
            font-weight: 700;
            margin: 0;
        }
        pre {
            margin: 0;
            padding: 14px;
            border-radius: 12px;
            background: #201a17;
            color: #f4eee7;
            font-family: var(--mono);
            overflow: auto;
            min-height: 220px;
        }
    </style>
</head>
<body>
    <main>
        <h1>Mycela IPC Desktop</h1>
        <p>This window is running without a loopback HTTP listener. It talks to the Rust host through the WebView IPC bridge.</p>
        <div class="actions">
            <button onclick="sendCommand('app_ping')">Ping Host</button>
            <button class="secondary" onclick="refreshStatuses()">Refresh Status</button>
            <button class="secondary" onclick="sendCommand('app_version_get')">App Version</button>
        </div>
        <div class="grid">
            <section class="card">
                <h2>EPICS</h2>
                <p id="epics-status" class="status">Unknown</p>
                <div class="actions">
                    <button onclick="sendLifecycle('epics_server_start', 'epics-status-')">Start</button>
                    <button class="secondary" onclick="sendLifecycle('epics_server_stop', 'epics-status-')">Stop</button>
                </div>
            </section>
            <section class="card">
                <h2>Modbus</h2>
                <p id="modbus-status" class="status">Unknown</p>
                <div class="actions">
                    <button onclick="sendLifecycle('modbus_sim_start', 'modbus-status-')">Start</button>
                    <button class="secondary" onclick="sendLifecycle('modbus_sim_stop', 'modbus-status-')">Stop</button>
                </div>
            </section>
        </div>
        <pre id="ipc-log">Waiting for IPC responses...</pre>
    </main>
    <script>
        const MYCELA_IPC_TOKEN = "__MYCELA_IPC_TOKEN__";

        function requestId() {
            if (window.crypto && typeof window.crypto.randomUUID === 'function') {
                return window.crypto.randomUUID();
            }
            return String(Date.now()) + '-' + String(Math.random()).slice(2);
        }

        function logResponse(response) {
            const log = document.getElementById('ipc-log');
            log.textContent = JSON.stringify(response, null, 2);
        }

        function updateStatuses(response) {
            if (!response.ok || !response.result) {
                return;
            }
            if (response.id.startsWith('epics-status-')) {
                document.getElementById('epics-status').textContent = response.result.running ? 'Running' : 'Stopped';
            }
            if (response.id.startsWith('modbus-status-')) {
                document.getElementById('modbus-status').textContent = response.result.running ? 'Running' : 'Stopped';
            }
        }

        function sendCommand(cmd, payload, customId) {
            const message = {
                v: 1,
                kind: 'request',
                id: customId || requestId(),
                cmd,
                token: MYCELA_IPC_TOKEN,
                payload: payload || {},
                ts: Date.now()
            };
            window.ipc.postMessage(JSON.stringify(message));
        }

        function refreshStatuses() {
            sendCommand('epics_server_status_get', {}, 'epics-status-' + requestId());
            sendCommand('modbus_sim_status_get', {}, 'modbus-status-' + requestId());
        }

        function sendLifecycle(cmd, prefix) {
            sendCommand(cmd, {}, prefix + requestId());
            window.setTimeout(refreshStatuses, 150);
        }

        window.__MYCELA_IPC_DELIVER = function(response) {
            logResponse(response);
            updateStatuses(response);
        };

        refreshStatuses();
    </script>
</body>
</html>"#
        .replace("\"__MYCELA_IPC_TOKEN__\"", &token_json)
}

fn build_app_state(config: AppConfig) -> AppState {
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

        let (sim_h, listener_h) = modbus_simulator::start_modbus_simulator(5020);
        tracing::info!("Modbus TCP simulator started on port 5020");
        let modbus_pool = modbus_client::ModbusPool::new();
        let channel_ctx = ChannelContext::new(epics_ctx, modbus_pool);

        AppState {
                pv_server,
                config: Arc::new(config),
                channel_ctx,
                modbus_task: Arc::new(Mutex::new(Some(vec![sim_h, listener_h]))),
            epics_start_hook: Some(Arc::new(|state, server| {
                for screen in &state.config.screens {
                    epics_simulator::start_demo_simulator(server.handle(), &screen.widgets);
                }
                Ok(())
            })),
            modbus_start_hook: Some(Arc::new(|_state| {
                let (sim_h, listener_h) = modbus_simulator::start_modbus_simulator(5020);
                Ok(vec![sim_h, listener_h])
            })),
        }
}

fn spawn_ipc_backend(config: AppConfig, session_token: String) -> mpsc::Sender<BackendRequest> {
        let (backend_tx, backend_rx) = mpsc::channel::<BackendRequest>();

        std::thread::spawn(move || {
                let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
                let state = runtime.block_on(async move { build_app_state(config) });

                while let Ok(backend_request) = backend_rx.recv() {
                        let response = runtime.block_on(ipc_dispatch::dispatch_request(
                                &state,
                                backend_request.request,
                        Some(&session_token),
                        ));
                        if let Err(error) = backend_request.response_tx.send(response) {
                                tracing::error!("Failed to send IPC response to UI thread: {}", error);
                        }
                }
        });

        backend_tx
}

fn generate_ipc_session_token() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    format!("ipc-{}-{}", std::process::id(), now)
}

// --- Entry point -------------------------------------------------------------

fn main() {
    let _log_guard = mycela::logging::init_logging(Some(std::path::Path::new("logs")));
    tracing::info!("Starting Mycela Desktop");

    let config: AppConfig = serde_json::from_str(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/demo_app.json")),
    )
    .expect("embedded demo_app.json is invalid");

    if let Ok(raw_transport) = std::env::var(DESKTOP_TRANSPORT_ENV) {
        if DesktopTransport::parse(&raw_transport).is_none() {
            tracing::warn!(
                "Unknown {} value '{}'; falling back to loopback",
                DESKTOP_TRANSPORT_ENV,
                raw_transport
            );
        }
    }

    let transport = DesktopTransport::from_env();
    tracing::info!(
        "Selected desktop transport: {} (set with {})",
        transport.as_str(),
        DESKTOP_TRANSPORT_ENV
    );

    match transport {
        DesktopTransport::Loopback => run_loopback_desktop(config),
        DesktopTransport::Ipc => run_ipc_desktop(config),
    }
}

fn run_ipc_desktop(config: AppConfig) {
    let session_token = generate_ipc_session_token();
    let backend_tx = spawn_ipc_backend(config, session_token.clone());

    let event_loop = EventLoop::<DesktopUserEvent>::with_user_event().build().unwrap();
    let proxy = event_loop.create_proxy();
    let mut app = DesktopApp {
        transport: DesktopTransport::Ipc,
        base_url: Some(build_ipc_shell_html(&session_token)),
        windows: Vec::new(),
        proxy,
        backend_tx: Some(backend_tx),
    };
    event_loop.run_app(&mut app).unwrap();
}

fn run_loopback_desktop(config: AppConfig) {

    // Channel: background server thread sends the bound port to the main thread.
    let (port_tx, port_rx) = std::sync::mpsc::channel::<u16>();

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        runtime.block_on(async move {
            let state = build_app_state(config);

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
    let event_loop = EventLoop::<DesktopUserEvent>::with_user_event().build().unwrap();
    let proxy = event_loop.create_proxy();
    let mut app = DesktopApp {
        transport: DesktopTransport::Loopback,
        base_url: Some(url),
        windows: Vec::new(),
        proxy,
        backend_tx: None,
    };
    event_loop.run_app(&mut app).unwrap();
}
