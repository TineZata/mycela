use maud::{html, Markup};
use crate::config::WidgetConfig;
use pvxs_sys::{Context, Value, MonitorEvent};

pub struct Select {
    config: WidgetConfig,
}

impl Select {
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
        tracing::info!("[{}] Select monitor starting for PV: {}", config.id, config.pv_name);

        let mut ctx = match Context::from_env() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("[{}] Context creation failed for {}: {}", config.id, config.pv_name, e);
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
                tracing::error!("[{}] Monitor creation failed for {}: {}", config.id, config.pv_name, e);
                let _ = tx.send(render_inner_disconnected(&config).into_string());
                return;
            }
        };

        if let Err(e) = monitor.start() {
            tracing::error!("[{}] Monitor start failed for {}: {}", config.id, config.pv_name, e);
            return;
        }

        let mut last_html = String::new();

        loop {
            match monitor.pop() {
                Ok(Some(raw)) => {
                    let current_index = raw.get_field_enum("value.index").unwrap_or(0);
                    tracing::debug!("[{}] monitor pop => index={}", config.id, current_index);
                    let html = render_inner_connected(&config, &raw).into_string();
                    if html != last_html {
                        tracing::debug!("[{}] HTML changed, sending SSE update", config.id);
                        last_html = html.clone();
                        if tx.send(html).is_err() {
                            tracing::info!("[{}] SSE receiver dropped (browser closed connection?)", config.id);
                            break;
                        }
                    } else {
                        tracing::debug!("[{}] HTML unchanged, skipping SSE", config.id);
                    }
                }
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
                Err(MonitorEvent::Connected(msg)) => {
                    tracing::info!("[{}] Select connected: {}", config.id, msg);
                }
                Err(MonitorEvent::Disconnected(msg)) => {
                    tracing::warn!("[{}] Select disconnected: {}", config.id, msg);
                    if tx.send(render_inner_disconnected(&config).into_string()).is_err() {
                        tracing::info!("[{}] SSE receiver dropped (browser closed connection?)", config.id);
                        break;
                    }
                }
                Err(MonitorEvent::Finished(msg)) => {
                    tracing::info!("[{}] Select PV finished: {}", config.id, msg);
                    break;
                }
                Err(MonitorEvent::RemoteError(msg) | MonitorEvent::ClientError(msg)) => {
                    tracing::error!("[{}] Select error: {}", config.id, msg);
                    if tx.send(render_inner_disconnected(&config).into_string()).is_err() {
                        tracing::info!("[{}] SSE receiver dropped (browser closed connection?)", config.id);
                        break;
                    }
                }
            }
        }

        tracing::info!("[{}] Select monitor stopped for PV: {}", config.id, config.pv_name);
    }
}

fn render_inner_connected(config: &WidgetConfig, raw: &Value) -> Markup {
    let alarm_severity = raw.get_field_int32("alarm.severity").unwrap_or(0);
    let alarm_class    = super::alarm_severity_class(alarm_severity);
    let icon: Option<&str> = match alarm_severity {
        1 => Some(super::MINOR_ALARM_SVG),
        2 => Some(super::MAJOR_ALARM_SVG),
        3 => Some(super::INVALID_SVG),
        _ => None,
    };

    // Enum index currently selected
    let current_index = raw.get_field_enum("value.index").unwrap_or(0) as usize;

    // Choices from value.choices as array of strings
    let choices = raw.get_field_string_array("value.choices").unwrap_or_default();
    let tooltip = super::build_tooltip(&config, raw);
    let display_text = choices.get(current_index).map(|s| s.trim().to_string())
        .unwrap_or_else(|| current_index.to_string());

    html! {
        div class="widget-inner" {
            label class="widget-label" {
                (config.label)
                @if !tooltip.is_empty() {
                    (super::render_info_btn(&tooltip))
                }
            }
            div class="select-with-icon-container" {
                @if let Some(src) = icon {
                    img class="select-icon" src=(src) alt="status";
                }
                div class="select-wrapper" {
                    select class=(format!("widget-select {}", alarm_class))
                        name="value"
                        hx-post={"/api/widget/" (config.id) "/set"}
                        hx-trigger="change"
                        hx-target="next .status"
                        hx-swap="innerHTML" {
                        @for (idx, choice) in choices.iter().enumerate() {
                            option value=(idx) selected[idx == current_index] { (choice.trim()) }
                        }
                        @if choices.is_empty() {
                            option value=(current_index) selected { (current_index) }
                        }
                    }
                    span class="select-display-text" { (display_text) }
                }
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

fn render_inner_disconnected(config: &WidgetConfig) -> Markup {
    html! {
        div class="widget-inner" {
            label class="widget-label" { (config.label) }
            div class="select-with-icon-container" {
                img class="select-icon" src=(super::OFFLINE_SVG) alt="offline";
                div class="select-wrapper" {
                    select class="widget-select alarm-disconnected" disabled {
                        option { "--" }
                    }
                    span class="select-display-text" { "--" }
                }
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

/// Render the outer SSE shell for a select widget.
pub fn render_select(widget: &WidgetConfig) -> Markup {
    html! {
        div style=[super::widget_container_style(widget)]
            data-widget-id=(widget.id)
            data-pv=(widget.pv_name)
            hx-sse=(format!("swap:{}", widget.id)) {
            (render_inner_disconnected(widget))
        }
    }
}
