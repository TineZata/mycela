#[path = "../demo_server/epics_simulator.rs"]
mod epics_simulator;
#[path = "../demo_server/modbus_simulator.rs"]
mod modbus_simulator;
mod assets;

use axum::{
    Router,
    routing::{get, post},
    extract::{Path, State, Form},
    response::{Html, IntoResponse, Response, sse::{Event, KeepAlive, Sse}},
    http::{StatusCode, header},
};
use std::sync::{Arc, Mutex};
use tower_http::{
    trace::TraceLayer,
    cors::CorsLayer,
};
use maud::{html, Markup};

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};
use wry::WebViewBuilder;

use mycela::channel::ChannelContext;
use mycela::config::{ProtocolConfig, ScreenConfig, WidgetConfig, WidgetType};
use mycela::server_setup::setup_server_pvs;
use mycela::{modbus_client, widgets};

// --- Application state -------------------------------------------------------

#[derive(Clone)]
struct AppState {
    pv_server:   Arc<Mutex<Option<pvxs_sys::Server>>>,
    config:      Arc<ScreenConfig>,
    channel_ctx: Arc<ChannelContext>,
    modbus_task: Arc<Mutex<Option<Vec<tokio::task::JoinHandle<()>>>>>,
}

impl AppState {
    fn is_server_running(&self) -> bool {
        self.pv_server.lock().unwrap().is_some()
    }
    fn is_modbus_running(&self) -> bool {
        self.modbus_task.lock().unwrap()
            .as_ref()
            .map(|v| v.iter().any(|h| !h.is_finished()))
            .unwrap_or(false)
    }
}

// --- Static file handler (embedded assets) -----------------------------------

async fn static_file_handler(Path(path): Path<String>) -> impl IntoResponse {
    match assets::get_asset(&path) {
        Some((bytes, content_type)) => {
            ([(header::CONTENT_TYPE, content_type)], bytes).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// --- Widget write endpoint ---------------------------------------------------

async fn write_widget(
    Path(widget_id): Path<String>,
    State(state): State<AppState>,
    Form(form): Form<widgets::WriteForm>,
) -> Response {
    let widget = widgets::collect_data_widgets(&state.config.widgets)
        .into_iter()
        .find(|w| w.id == widget_id);
    match widget {
        None => (StatusCode::NOT_FOUND, Html(format!(
            "<span class=\"write-err\">Widget '{}' not found</span>", widget_id
        ))).into_response(),
        Some(w) => Html(
            widgets::write_channel(w, form.value, state.channel_ctx.clone())
                .await
                .into_string(),
        ).into_response(),
    }
}

// --- Page handler ------------------------------------------------------------

async fn render_demo_screen(State(state): State<AppState>) -> Html<String> {
    tracing::info!("Rendering widget showcase");
    let markup = render_showcase(
        &state.config,
        state.is_server_running(),
        state.is_modbus_running(),
    );
    Html(markup.into_string())
}

// --- SSE handlers ------------------------------------------------------------

type SseStream = std::pin::Pin<
    Box<dyn tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>> + Send>,
>;

async fn stream_widget(
    Path(widget_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    tracing::info!("SSE stream requested for widget: {}", widget_id);

    let data_widgets = widgets::collect_data_widgets(&state.config.widgets);
    let Some(config) = data_widgets.into_iter().find(|w| w.id == widget_id) else {
        tracing::error!("Widget '{}' not found", widget_id);
        let stream: SseStream = Box::pin(async_stream::stream! {
            yield Ok(Event::default().data("<!-- widget not found -->"));
        });
        return Sse::new(stream).keep_alive(KeepAlive::default());
    };

    let ctx = state.channel_ctx.clone();
    let stream: SseStream = match config.widget_type {
        WidgetType::TextEntry    => Box::pin(widgets::text_entry::TextEntry::new(config).into_sse_stream(ctx)),
        WidgetType::TextUpdate   => Box::pin(widgets::text_update::TextUpdate::new(config).into_sse_stream(ctx)),
        WidgetType::Gauge        => Box::pin(widgets::gauge::Gauge::new(config).into_sse_stream(ctx)),
        WidgetType::Led          => Box::pin(widgets::led::Led::new(config).into_sse_stream(ctx)),
        WidgetType::Slider       => Box::pin(widgets::slider::Slider::new(config).into_sse_stream(ctx)),
        WidgetType::Button       => Box::pin(widgets::button::Button::new(config).into_sse_stream(ctx)),
        WidgetType::ToggleButton => Box::pin(widgets::toggle_button::ToggleButton::new(config).into_sse_stream(ctx)),
        WidgetType::Chart        => Box::pin(widgets::chart::Chart::new(config).into_sse_stream(ctx)),
        WidgetType::Select       => Box::pin(widgets::select::Select::new(config).into_sse_stream(ctx)),
        WidgetType::Group        => {
            let stream: SseStream = Box::pin(async_stream::stream! {
                yield Ok(Event::default().data("<!-- group widget has no stream -->"));
            });
            return Sse::new(stream).keep_alive(KeepAlive::default());
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn stream_all_widgets(State(state): State<AppState>) -> impl IntoResponse {
    tracing::info!("Multiplexed SSE stream requested for all widgets");

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(String, String)>();

    let data_widgets = widgets::collect_data_widgets(&state.config.widgets);
    for config in data_widgets {
        let tx        = tx.clone();
        let widget_id = config.id.clone();
        let ctx       = state.channel_ctx.clone();
        tokio::spawn(widgets::run_widget_monitor_async(config, widget_id, ctx, tx));
    }
    drop(tx);

    let stream: SseStream = Box::pin(async_stream::stream! {
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
        setup_server_pvs(&server, &config.widgets)?;
        pvxs_sys::Result::Ok(server)
    })
    .await;

    match result {
        Ok(Ok(server)) => {
            epics_simulator::start_demo_simulator(server.handle(), &state.config.widgets);
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
                            span { "Server Stopped" }
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

async fn server_status(State(state): State<AppState>) -> Html<String> {
    let is_running = state.is_server_running();
    let html = maud::html! {
        div id="server-status" class=(if is_running { "success" } else { "warning" }) {
            span { @if is_running { "Server Running" } @else { "Server Stopped" } }
        }
    };
    Html(html.into_string())
}

async fn modbus_status(State(state): State<AppState>) -> Html<String> {
    let is_running = state.is_modbus_running();
    let html = maud::html! {
        div id="modbus-status" class=(if is_running { "success" } else { "warning" }) {
            span { @if is_running { "Modbus TCP Running" } @else { "Modbus TCP Stopped" } }
        }
    };
    Html(html.into_string())
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

async fn stop_modbus(State(state): State<AppState>) -> Response {
    tracing::info!("POST /api/modbus/stop");
    let handles = state.modbus_task.lock().unwrap().take();
    match handles {
        None => {
            let html = maud::html! { div class="warning" { "Modbus TCP simulator is not running" } };
            (StatusCode::BAD_REQUEST, Html(html.into_string())).into_response()
        }
        Some(handles) => {
            for h in handles { h.abort(); }
            state.channel_ctx.modbus_pool.disconnect_all();
            tracing::info!("Modbus TCP demo simulator stopped");
            let html = maud::html! {
                div id="modbus-status" class="warning" hx-swap-oob="true" {
                    span { "Modbus TCP Stopped" }
                }
            };
            Html(html.into_string()).into_response()
        }
    }
}

// --- Showcase page -----------------------------------------------------------

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

fn render_showcase(config: &ScreenConfig, server_running: bool, modbus_running: bool) -> Markup {
    fn proto_label(w: &WidgetConfig) -> &'static str {
        match &w.protocol {
            Some(ProtocolConfig::EpicsPva(_)) | None => "EPICS PVA",
            Some(ProtocolConfig::ModbusTcp(_))       => "Modbus TCP",
            _ => "Unknown",
        }
    }

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

    let mut pairs: Vec<(String, WidgetType, &WidgetConfig, Option<&WidgetConfig>)> = Vec::new();
    for widget in data_widgets {
        let key = match widget.widget_type {
            WidgetType::Chart => format!("Chart_{}", widget.id),
            _ => format!("{:?}_{}", widget.widget_type, proto_label(widget)),
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
                title { "Widget Showcase: " (config.title) }
                script src="/static/htmx.min.js" {}
                script src="/static/tooltip.js" {}
                link rel="stylesheet" href="/static/style.css";
                style { "body { background: linear-gradient(135deg, #1a1a1a 0%, #2d2d2d 100%); }" }
            }
            body {
                header class="main-header" {
                    h1 { (config.title) }
                    div class="server-controls" style="display:flex;flex-direction:column;gap:0.5rem;margin-top:0.5rem;" {
                        div style="display:flex;align-items:center;gap:1rem;" {
                            span style="min-width:9rem;font-size:0.8rem;color:var(--text-secondary);" { "EPICS PVA" }
                            div id="server-status" hx-get="/api/server/status" hx-trigger="load, every 2s" {
                                @if server_running {
                                    span class="success" style="display:flex;align-items:center;gap:0.4rem;" {
                                        img src=(widgets::CHECK_CIRCLE_SVG) alt="running" style="width:20px;height:20px;";
                                        "Running"
                                    }
                                } @else {
                                    span class="warning" style="display:flex;align-items:center;gap:0.4rem;color:var(--alarm-minor)" {
                                        img src=(widgets::CANCEL_SVG) alt="stopped" style="width:20px;height:20px;";
                                        "Stopped"
                                    }
                                }
                            }
                            button class="widget-button" hx-post="/api/server/start" hx-target="#server-status"
                                style="padding:0.4rem 0.9rem;font-size:0.85rem;" { "Start" }
                            button class="widget-button" hx-post="/api/server/stop" hx-target="#server-status"
                                style="padding:0.4rem 0.9rem;font-size:0.85rem;background:#dc3545;" { "Stop" }
                        }
                        div style="display:flex;align-items:center;gap:1rem;" {
                            span style="min-width:9rem;font-size:0.8rem;color:var(--text-secondary);" { "Modbus TCP" }
                            div id="modbus-status" hx-get="/api/modbus/status" hx-trigger="load, every 2s" {
                                @if modbus_running {
                                    span class="success" style="display:flex;align-items:center;gap:0.4rem;" {
                                        img src=(widgets::CHECK_CIRCLE_SVG) alt="running" style="width:20px;height:20px;";
                                        "Running"
                                    }
                                } @else {
                                    span class="warning" style="display:flex;align-items:center;gap:0.4rem;color:var(--alarm-minor)" {
                                        img src=(widgets::CANCEL_SVG) alt="stopped" style="width:20px;height:20px;";
                                        "Stopped"
                                    }
                                }
                            }
                            button class="widget-button" hx-post="/api/modbus/start" hx-target="#modbus-status"
                                style="padding:0.4rem 0.9rem;font-size:0.85rem;" { "Start" }
                            button class="widget-button" hx-post="/api/modbus/stop" hx-target="#modbus-status"
                                style="padding:0.4rem 0.9rem;font-size:0.85rem;background:#dc3545;" { "Stop" }
                        }
                    }
                }

                main class="showcase-page" hx-sse="connect:/stream/all" {
                    p class="showcase-description" { (config.description) }

                    @for (_key, wtype, dark_w, light_w) in &pairs {
                        @let first_proto = proto_label(dark_w);
                        section class="widget-section" {
                            div class="section-header" {
                                span class="widget-type-badge" { (widget_type_name(wtype)) }
                                @let proto_color = if first_proto == "Modbus TCP" { "#f0a500" } else { "#5b8dd9" };
                                span style=(format!("font-size:0.7rem;padding:0.15rem 0.5rem;border-radius:0.9rem;background:{};color:#fff;margin-left:0.4rem;", proto_color)) {
                                    (first_proto)
                                }
                                p class="section-description" {
                                    @if let Some(desc) = &dark_w.description { (desc) }
                                }
                            }
                            div class={"theme-pair" @if *wtype == WidgetType::Chart { " theme-pair--chart" }} {
                                div class="mockup-card mockup-card--dark" {
                                    div class="mockup-card__titlebar" {
                                        span class="theme-dot" {}
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
                                                    "no light widget configured"
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

// --- Desktop window ----------------------------------------------------------

struct DesktopApp {
    url:     String,
    window:  Option<Window>,
    webview: Option<wry::WebView>,
}

impl ApplicationHandler for DesktopApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = event_loop
            .create_window(Window::default_attributes().with_title("Mycela"))
            .expect("failed to create window");
        let webview = WebViewBuilder::new()
            .with_url(&self.url)
            .build(&window)
            .expect("failed to create webview");
        self.window  = Some(window);
        self.webview = Some(webview);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        if matches!(event, WindowEvent::CloseRequested) {
            event_loop.exit();
        }
    }
}

// --- Entry point -------------------------------------------------------------

fn main() {
    let _log_guard = mycela::logging::init_logging(Some(std::path::Path::new("logs")));
    tracing::info!("Starting Mycela Desktop");

    let config: ScreenConfig = serde_json::from_str(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/demo_config.json")),
    )
    .expect("embedded demo_config.json is invalid");

    // Channel: background server thread sends the bound port to the main thread.
    let (port_tx, port_rx) = std::sync::mpsc::channel::<u16>();

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        runtime.block_on(async move {
            // EPICS / PVXS setup
            let data_widgets = widgets::collect_data_widgets(&config.widgets);
            let pv_server = {
                let server_widgets: Vec<_> = data_widgets
                    .iter()
                    .filter(|w| w.epics_pva().and_then(|e| e.server.as_ref()).is_some())
                    .collect();
                if server_widgets.is_empty() {
                    tracing::info!("No server PVs configured, running in client-only mode");
                    Arc::new(Mutex::new(None))
                } else {
                    let server = pvxs_sys::Server::start_from_env()
                        .expect("PVXS server start");
                    setup_server_pvs(&server, &config.widgets).expect("PVXS setup");
                    tracing::info!("PVXS server started");
                    epics_simulator::start_demo_simulator(server.handle(), &config.widgets);
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

            let app = Router::new()
                .route("/",                              get(render_demo_screen))
                .route("/api/server/start",              post(start_server))
                .route("/api/server/stop",               post(stop_server))
                .route("/api/server/status",             get(server_status))
                .route("/api/modbus/start",              post(start_modbus))
                .route("/api/modbus/stop",               post(stop_modbus))
                .route("/api/modbus/status",             get(modbus_status))
                .route("/api/widget/{widget_id}/set",    post(write_widget))
                .route("/stream/widget/{name}",          get(stream_widget))
                .route("/stream/all",                    get(stream_all_widgets))
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
    let event_loop = EventLoop::new().unwrap();
    let mut app = DesktopApp { url, window: None, webview: None };
    event_loop.run_app(&mut app).unwrap();
}
