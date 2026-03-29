use maud::{html, Markup};
use crate::config::WidgetConfig;
use pvxs_sys::{Context, Value, MonitorEvent};

pub struct Gauge {
    config: WidgetConfig,
}

impl Gauge {
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
        tracing::info!("Gauge monitor starting for: {}", config.pv_name);

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
                    tracing::info!("Gauge {}: connected - {}", config.pv_name, msg);
                }
                Err(MonitorEvent::Disconnected(msg)) => {
                    tracing::warn!("Gauge {}: disconnected - {}", config.pv_name, msg);
                    if tx.send(render_inner_disconnected(&config).into_string()).is_err() { break; }
                }
                Err(MonitorEvent::Finished(msg)) => {
                    tracing::info!("Gauge {}: finished - {}", config.pv_name, msg);
                    break;
                }
                Err(MonitorEvent::RemoteError(msg) | MonitorEvent::ClientError(msg)) => {
                    tracing::error!("Gauge {}: error - {}", config.pv_name, msg);
                    if tx.send(render_inner_disconnected(&config).into_string()).is_err() { break; }
                }
            }
        }

        tracing::info!("Gauge monitor stopped for: {}", config.pv_name);
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

    let min = raw.get_field_double("display.limitLow").unwrap_or(0.0);
    let max = raw.get_field_double("display.limitHigh").unwrap_or(100.0);
    let max = if (max - min).abs() < f64::EPSILON { min + 100.0 } else { max };
    let percentage = ((current_value - min) / (max - min) * 100.0).clamp(0.0, 100.0);

    // Alarm limit markers: (actual_value, bar_percentage)
    let range = max - min;
    let to_pct = |v: f64| ((v - min) / range * 100.0).clamp(0.0, 100.0);
    let low_alarm  = raw.get_field_double("valueAlarm.lowAlarmLimit").ok().map(|v| (v, to_pct(v)));
    let low_warn   = raw.get_field_double("valueAlarm.lowWarningLimit").ok().map(|v| (v, to_pct(v)));
    let high_warn  = raw.get_field_double("valueAlarm.highWarningLimit").ok().map(|v| (v, to_pct(v)));
    let high_alarm = raw.get_field_double("valueAlarm.highAlarmLimit").ok().map(|v| (v, to_pct(v)));

    render_gauge_html(config, &display_value, &units, min, max, percentage,
                      &format!("gauge {}", alarm_class), icon,
                      low_alarm, low_warn, high_warn, high_alarm,
                      &super::build_tooltip(&config, raw))
}

fn render_inner_disconnected(config: &WidgetConfig) -> Markup {
    render_gauge_html(config, "--", "", 0.0, 100.0, 0.0, "gauge alarm-disconnected", Some(super::OFFLINE_SVG),
                      None, None, None, None, "")
}

fn render_gauge_html(
    config: &WidgetConfig,
    display_value: &str,
    units: &str,
    min: f64,
    max: f64,
    percentage: f64,
    _alarm_class: &str,
    icon: Option<&str>,
    low_alarm:  Option<(f64, f64)>,
    low_warn:   Option<(f64, f64)>,
    high_warn:  Option<(f64, f64)>,
    high_alarm: Option<(f64, f64)>,
    tooltip: &str,
) -> Markup {
    let has_alarm_labels = low_alarm.is_some() || low_warn.is_some()
        || high_warn.is_some() || high_alarm.is_some();
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
            div class="gauge-display" {
                div class="gauge-value" {
                    (display_value)
                    @if !units.is_empty() { " " (units) }
                }
                // bar + alarm marker overlay
                div class="gauge-bar" {
                    div class="gauge-fill" style=(format!("width: {:.1}%", percentage)) {}
                    @if let Some((_, p)) = low_alarm {
                        div class="gauge-marker gauge-marker--alarm" style=(format!("left:{:.2}%", p)) {}
                    }
                    @if let Some((_, p)) = low_warn {
                        div class="gauge-marker gauge-marker--warn" style=(format!("left:{:.2}%", p)) {}
                    }
                    @if let Some((_, p)) = high_warn {
                        div class="gauge-marker gauge-marker--warn" style=(format!("left:{:.2}%", p)) {}
                    }
                    @if let Some((_, p)) = high_alarm {
                        div class="gauge-marker gauge-marker--alarm" style=(format!("left:{:.2}%", p)) {}
                    }
                }
                @if has_alarm_labels {
                    div class="gauge-labels" {
                        @if let Some((v, p)) = low_alarm {
                            span class="gauge-limit gauge-limit--low-low" style=(format!("left:{:.2}%", p)) { (format!("{:.1}", v)) }
                        }
                        @if let Some((v, p)) = low_warn {
                            span class="gauge-limit gauge-limit--low" style=(format!("left:{:.2}%", p)) { (format!("{:.1}", v)) }
                        }
                        @if let Some((v, p)) = high_warn {
                            span class="gauge-limit gauge-limit--high" style=(format!("left:{:.2}%", p)) { (format!("{:.1}", v)) }
                        }
                        @if let Some((v, p)) = high_alarm {
                            span class="gauge-limit gauge-limit--high-high" style=(format!("left:{:.2}%", p)) { (format!("{:.1}", v)) }
                        }
                    }
                } @else {
                    div class="gauge-range" {
                        span class="min" { (format!("{:.1}", min)) }
                        span class="max" { (format!("{:.1}", max)) }
                    }
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

pub fn render_gauge(widget: &WidgetConfig) -> Markup {
    html! {
        div data-widget-id=(widget.id)
            data-pv=(widget.pv_name)
            sse-swap=(widget.id)
            hx-swap="innerHTML" {
            (render_inner_disconnected(widget))
        }
    }
}


