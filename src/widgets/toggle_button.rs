use maud::{html, Markup};
use crate::config::WidgetConfig;
use pvxs_sys::{Context, Value, MonitorEvent};

pub struct ToggleButton {
    config: WidgetConfig,
}

impl ToggleButton {
    pub fn new(config: WidgetConfig) -> Self {
        Self { config }
    }

    pub fn into_sse_stream(
        self,
    ) -> impl tokio_stream::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>
           + Send
           + 'static {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let config = std::sync::Arc::new(self.config);
        let config_thread = config.clone();

        tokio::task::spawn_blocking(move || Self::run_monitor(config_thread, tx));

        async_stream::stream! {
            yield Ok(axum::response::sse::Event::default().data(
                render_inner_disconnected(&config).into_string()
            ));
            let mut rx = rx;
            while let Some(html) = rx.recv().await {
                yield Ok(axum::response::sse::Event::default().data(html));
            }
        }
    }

    pub(crate) fn run_monitor(
        config: std::sync::Arc<WidgetConfig>,
        tx: tokio::sync::mpsc::UnboundedSender<String>,
    ) {
        tracing::info!("ToggleButton monitor starting for: {}", config.pv_name);

        let mut ctx = match Context::from_env() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Context creation failed for {}: {}", config.pv_name, e);
                let _ = tx.send(render_inner_disconnected(&config).into_string());
                return;
            }
        };

        let mut monitor = match ctx
            .monitor_builder(&config.pv_name)
            .and_then(|b| b.connect_exception(true).disconnect_exception(true).exec())
        {
            Ok(m) => m,
            Err(e) => {
                tracing::error!("Monitor creation failed for {}: {}", config.pv_name, e);
                let _ = tx.send(render_inner_disconnected(&config).into_string());
                return;
            }
        };

        if let Err(e) = monitor.start() {
            tracing::error!("Monitor start failed for {}: {}", config.pv_name, e);
            return;
        }

        loop {
            match monitor.pop() {
                Ok(Some(raw)) => {
                    let html = render_inner_connected(&config, &raw).into_string();
                    if tx.send(html).is_err() { break; }
                }
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
                Err(MonitorEvent::Connected(msg)) => {
                    tracing::info!("ToggleButton {}: connected - {}", config.pv_name, msg);
                }
                Err(MonitorEvent::Disconnected(msg)) => {
                    tracing::warn!("ToggleButton {}: disconnected - {}", config.pv_name, msg);
                    if tx.send(render_inner_disconnected(&config).into_string()).is_err() { break; }
                }
                Err(MonitorEvent::Finished(msg)) => {
                    tracing::info!("ToggleButton {}: finished - {}", config.pv_name, msg);
                    break;
                }
                Err(MonitorEvent::RemoteError(msg) | MonitorEvent::ClientError(msg)) => {
                    tracing::error!("ToggleButton {}: error - {}", config.pv_name, msg);
                    if tx.send(render_inner_disconnected(&config).into_string()).is_err() { break; }
                }
            }
        }

        tracing::info!("ToggleButton monitor stopped for: {}", config.pv_name);
    }
}

fn render_inner_connected(config: &WidgetConfig, raw: &Value) -> Markup {
    let current = raw.get_field_int32("value").unwrap_or(0);
    let is_on = current != 0;
    let next_val = if is_on { "0" } else { "1" };

    let alarm_severity = raw.get_field_int32("alarm.severity").unwrap_or(0);
    let icon: Option<&str> = match alarm_severity {
        1 => Some(super::MINOR_ALARM_SVG),
        2 => Some(super::MAJOR_ALARM_SVG),
        3 => Some(super::INVALID_SVG),
        _ => None,
    };

    render_toggle_html(config, is_on, next_val, icon, false, &super::build_tooltip(config, raw))
}

fn render_inner_disconnected(config: &WidgetConfig) -> Markup {
    render_toggle_html(config, false, "0", Some(super::OFFLINE_SVG), true, "")
}

fn render_toggle_html(
    config: &WidgetConfig,
    is_on: bool,
    next_val: &str,
    icon: Option<&str>,
    disabled: bool,
    tooltip: &str,
) -> Markup {
    let btn_class = if is_on {
        "pv-button pv-toggle-btn pv-toggle-btn--on"
    } else {
        "pv-button pv-toggle-btn pv-toggle-btn--off"
    };
    let state_label = if is_on { "ON" } else { "OFF" };

    html! {
        div class="widget-inner" {
            @if !tooltip.is_empty() {
                div class="button-label-row" style="display:flex;align-items:center;gap:0.4rem;margin-bottom:0.5rem;" {
                    span class="widget-label" { (config.label) }
                    (super::render_info_btn(tooltip))
                }
            }
            button class=(btn_class)
                disabled[disabled]
                hx-post={"/api/widget/" (config.id) "/set"}
                hx-vals=(format!(r#"{{"value": "{}"}}"#, next_val))
                hx-target="next .status"
                hx-swap="innerHTML" {
                @if let Some(src) = icon {
                    img class="button-icon" src=(src) alt="status";
                }
                (config.label) " — " (state_label)
            }
            span class="status" {}
            @if let Some(desc) = &config.description {
                @if !desc.is_empty() {
                    p class="widget-description" { (desc) }
                }
            }
        }
    }
}

pub fn render_toggle_button(widget: &WidgetConfig) -> Markup {
    html! {
        div data-widget-id=(widget.id)
            data-pv=(widget.pv_name)
            sse-swap=(widget.id)
            hx-swap="innerHTML" {
            (render_inner_disconnected(widget))
        }
    }
}
