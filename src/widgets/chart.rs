use maud::{html, Markup};
use crate::config::WidgetConfig;
use pvxs_sys::{Context, Value, MonitorEvent};

pub struct Chart {
    config: WidgetConfig,
}

impl Chart {
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
        tracing::info!("Chart monitor starting for: {}", config.pv_name);

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
                    tracing::info!("Chart {}: connected - {}", config.pv_name, msg);
                }
                Err(MonitorEvent::Disconnected(msg)) => {
                    tracing::warn!("Chart {}: disconnected - {}", config.pv_name, msg);
                    if tx.send(render_inner_disconnected(&config).into_string()).is_err() { break; }
                }
                Err(MonitorEvent::Finished(msg)) => {
                    tracing::info!("Chart {}: finished - {}", config.pv_name, msg);
                    break;
                }
                Err(MonitorEvent::RemoteError(msg) | MonitorEvent::ClientError(msg)) => {
                    tracing::error!("Chart {}: error - {}", config.pv_name, msg);
                    if tx.send(render_inner_disconnected(&config).into_string()).is_err() { break; }
                }
            }
        }

        tracing::info!("Chart monitor stopped for: {}", config.pv_name);
    }
}

fn render_inner_connected(config: &WidgetConfig, raw: &Value) -> Markup {
    let alarm_severity = raw.get_field_int32("alarm.severity").unwrap_or(0);
    let alarm_class = super::alarm_severity_class(alarm_severity);
    let icon: Option<&str> = match alarm_severity {
        1 => Some(super::MINOR_ALARM_SVG),
        2 => Some(super::MAJOR_ALARM_SVG),
        3 => Some(super::INVALID_SVG),
        _ => None,
    };

    let current_value = raw.get_field_double("value").unwrap_or(0.0);
    let prec = raw.get_field_int32("display.precision").unwrap_or(2);
    let display_value = format!("{:.prec$}", current_value, prec = prec as usize);
    let units = raw.get_field_string("display.units").unwrap_or_default();

    render_chart_html(config, &display_value, &units, &format!("chart {}", alarm_class), icon, &super::build_tooltip(&config, raw))
}

fn render_inner_disconnected(config: &WidgetConfig) -> Markup {
    render_chart_html(config, "--", "", "chart alarm-disconnected", Some(super::OFFLINE_SVG), "")
}

fn render_chart_html(
    config: &WidgetConfig,
    display_value: &str,
    units: &str,
    _alarm_class: &str,
    icon: Option<&str>,
    tooltip: &str,
) -> Markup {
    html! {
        div class="widget-inner" {
            label class="widget-label" {
                (config.label)
                @if let Some(src) = icon {
                    img class="widget-status-icon" src=(src) alt="status";
                }
                @if !tooltip.is_empty() {
                    (super::render_info_btn(tooltip))
                }
            }
            div class="chart-container" {
                canvas class="chart-canvas" width="400" height="200" {
                    "Chart: " (display_value)
                    @if !units.is_empty() { " " (units) }
                }
            }
            @if let Some(desc) = &config.description {
                @if !desc.is_empty() {
                    p class="widget-description" { (desc) }
                }
            }
        }
    }
}

pub fn render_chart(widget: &WidgetConfig) -> Markup {
    html! {
        div style=[super::widget_container_style(widget)]
            data-widget-id=(widget.id)
            data-pv=(widget.pv_name)
            hx-sse=(format!("swap:{}", widget.id)) {
            (render_inner_disconnected(widget))
        }
    }
}

