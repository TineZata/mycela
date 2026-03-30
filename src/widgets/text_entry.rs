use maud::{html, Markup};
use crate::config::WidgetConfig;
use pvxs_sys::{Context, Value, MonitorEvent};

// TextEntry widget struct
//
// Owns its PVXS Context and monitor thread entirely.
// pvxs_sys::Value is not Send/Clone, so the monitor thread renders HTML
// directly from the raw Value and sends String over the channel.
// PvValue/ConnectionStatus kept only for the public free functions used by other widgets.

pub struct TextEntry {
    config: WidgetConfig,
}

impl TextEntry {
    pub fn new(config: WidgetConfig) -> Self {
        Self { config }
    }

    /// Spawn a dedicated PVXS monitor and return a live SSE event stream.
    ///
    /// The monitor thread owns pvxs_sys::Context + Monitor and renders HTML
    /// directly from each pvxs_sys::Value. Only String crosses the thread
    /// boundary - no intermediate struct, no serialisation.
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
                render_inner_disconnected(&config, "Connecting...").into_string()
            ));
            let mut rx = rx;
            while let Some(html) = rx.recv().await {
                yield Ok(axum::response::sse::Event::default().data(html));
            }
        }
    }

    // PVXS monitor - renders HTML directly from pvxs_sys::Value

    pub(crate) fn run_monitor(
        config: std::sync::Arc<WidgetConfig>,
        tx: tokio::sync::mpsc::UnboundedSender<String>,
    ) {
        tracing::info!("TextEntry monitor starting for: {}", config.pv_name);

        let mut ctx = match Context::from_env() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Context creation failed for {}: {}", config.pv_name, e);
                let _ = tx.send(render_inner_disconnected(&config, &e.to_string()).into_string());
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
                let _ = tx.send(render_inner_disconnected(&config, &e.to_string()).into_string());
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
                    if tx.send(html).is_err() { break; } // browser disconnected
                }
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
                Err(MonitorEvent::Connected(msg)) => {
                    tracing::info!("TextEntry {}: connected - {}", config.pv_name, msg);
                }
                Err(MonitorEvent::Disconnected(msg)) => {
                    tracing::warn!("TextEntry {}: disconnected - {}", config.pv_name, msg);
                    if tx.send(render_inner_disconnected(&config, "PV Disconnected").into_string()).is_err() { break; }
                }
                Err(MonitorEvent::Finished(msg)) => {
                    tracing::info!("TextEntry {}: finished - {}", config.pv_name, msg);
                    break;
                }
                Err(MonitorEvent::RemoteError(msg) | MonitorEvent::ClientError(msg)) => {
                    tracing::error!("TextEntry {}: error - {}", config.pv_name, msg);
                    if tx.send(render_inner_disconnected(&config, &msg).into_string()).is_err() { break; }
                }
            }
        }

        tracing::info!("TextEntry monitor stopped for: {}", config.pv_name);
    }
}

fn render_inner_connected(config: &WidgetConfig, raw: &Value) -> Markup {
    let alarm_severity = raw.get_field_int32("alarm.severity").unwrap_or(0);
    let alarm_class    = super::alarm_severity_class(alarm_severity);
    let icon: Option<&str> = match pvxs_sys::AlarmSeverity::from(alarm_severity) {
        pvxs_sys::AlarmSeverity::NoAlarm => None,
        pvxs_sys::AlarmSeverity::Minor => Some(super::MINOR_ALARM_SVG),
        pvxs_sys::AlarmSeverity::Major => Some(super::MAJOR_ALARM_SVG),
        pvxs_sys::AlarmSeverity::Invalid => Some(super::INVALID_SVG),
        _ => Some(super::INVALID_SVG),
    };

    let is_integer = matches!(config.data_type.as_deref(), Some("integer") | Some("int") | Some("i32") | Some("int32") | Some("bool"));
    let is_enum = matches!(config.data_type.as_deref(), Some("enum"));
    let is_double = matches!(config.data_type.as_deref(), Some("double") | Some("float") | Some("f64") | Some("f32"));
    let mut is_string = false;
    let current_value = if is_integer {
        is_string = false;
        raw.get_field_int32("value").ok().map(|v| v.to_string())
    } else if is_double {
        is_string = false;
        let prec = raw.get_field_int32("display.precision").unwrap_or(2);
        raw.get_field_double("value").ok()
            .map(|v| format!("{:.prec$}", v, prec = prec as usize))
    } else if is_enum {
        // Returns the choice string from the enum value
        is_string = true;
        let enum_choices = raw.get_field_string("control.enumStrs").unwrap_or_default();
        let enum_value = raw.get_field_enum("value").ok();
        enum_value.map(|v| enum_choices.split(',').nth(v as usize).unwrap_or("").to_string())
    } else {
        raw.get_field_string("value").ok()
    }.unwrap_or_else(|| "??".to_string());

    let units    = raw.get_field_string("display.units").unwrap_or_default();
    let min_step = raw.get_field_double("control.minStep").unwrap_or(0.01);
    let tooltip  = super::build_tooltip(&config, raw);

    render_input_html(config, &current_value, &units, min_step, is_string,
                      &format!("text-entry {}", alarm_class), icon, false, &tooltip)
}

fn render_inner_disconnected(config: &WidgetConfig, _reason: &str) -> Markup {
    let is_string = config.data_type.as_deref() == Some("string");
    render_input_html(config, "--", "", 0.01, is_string,
                      "text-entry alarm-disconnected", Some(super::OFFLINE_SVG), true, "")
}

fn render_input_html(
    config: &WidgetConfig,
    current_value: &str,
    units: &str,
    min_step: f64,
    is_string: bool,
    input_class: &str,
    icon: Option<&str>,
    disabled: bool,
    tooltip: &str,
) -> Markup {
    let input_type = if is_string { "text" } else { "number" };
    html! {
        div class="widget-inner" {
            label class="widget-label" {
                (config.label)
                @if !tooltip.is_empty() {
                    button class="widget-info-btn" data-tooltip=(tooltip) type="button" {
                        img class="info-icon info-icon--dark"  src=(super::INFO_SVG_DARK)  alt="info";
                        img class="info-icon info-icon--light" src=(super::INFO_SVG_LIGHT) alt="info";
                    }                }
            }
            div class="text-entry-with-icon-container" {
                @if let Some(src) = icon {
                    img class="text-entry-icon" src=(src) alt="status";
                }
                @if is_string {
                    input type="text"
                        class=(input_class)
                        name="value"
                        value=(current_value)
                        disabled[disabled]
                        hx-post={"/api/widget/" (config.id) "/set"}
                        hx-trigger="keyup[key=='Enter']"
                        hx-target="next .status"
                        hx-swap="innerHTML";
                } @else {
                    input type=(input_type)
                        class=(input_class)
                        name="value"
                        value=(current_value)
                        data-original-value=(current_value)
                        step=(format!("{}", min_step))
                        disabled[disabled]
                        hx-post={"/api/widget/" (config.id) "/set"}
                        hx-trigger="keyup[key=='Enter']"
                        hx-target="next .status"
                        hx-swap="innerHTML"
                        hx-on--before-request="if(isNaN(parseFloat(this.value))||!isFinite(this.value)){this.value=this.dataset.originalValue;event.preventDefault();this.parentElement.nextElementSibling.textContent='Invalid number';return false;}else{this.dataset.originalValue=this.value;this.parentElement.nextElementSibling.textContent='';return true;}";
                }
                @if !units.is_empty() {
                    span class="units-overlay" { (units) }
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

/// Render the outer SSE shell for a text entry widget.
/// The monitor immediately pushes the first update via SSE — no initial value needed.
pub fn render_text_entry(widget: &WidgetConfig) -> Markup {
    html! {
        div style=[super::widget_container_style(widget)]
            data-widget-id=(widget.id)
            data-pv=(widget.pv_name)
            hx-sse=(format!("swap:{}", widget.id)) {
            (render_inner_disconnected(widget, "Connecting..."))
        }
    }
}
