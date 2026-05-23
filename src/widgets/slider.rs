use maud::{html, Markup};
use std::sync::Arc;
use futures::StreamExt;
use crate::channel::{ChannelContext, ChannelEvent, ChannelValue};
use crate::config::WidgetConfig;

pub struct Slider {
    config: WidgetConfig,
}

impl Slider {
    pub fn new(config: WidgetConfig) -> Self {
        Self { config }
    }

    pub fn into_sse_stream(
        self,
        ctx: Arc<ChannelContext>,
    ) -> impl tokio_stream::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>
           + Send
           + 'static {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let config = Arc::new(self.config);

        tokio::spawn(Self::run_monitor_async(config.clone(), ctx, tx));

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

    pub(crate) async fn run_monitor_async(
        config: Arc<WidgetConfig>,
        ctx: Arc<ChannelContext>,
        tx: tokio::sync::mpsc::UnboundedSender<String>,
    ) {
        let mut stream = crate::channel::channel_stream(config.clone(), ctx);
        while let Some(event) = stream.next().await {
            let html = match event {
                ChannelEvent::Value(cv)          => render_inner_connected(&config, &cv).into_string(),
                ChannelEvent::Disconnected(_)
                | ChannelEvent::Error(_)         => render_inner_disconnected(&config).into_string(),
                ChannelEvent::Connected          => continue,
            };
            if tx.send(html).is_err() { break; }
        }
    }
}

pub fn render_inner_connected(config: &WidgetConfig, cv: &ChannelValue) -> Markup {
    let alarm_class = super::alarm_severity_class(cv.alarm_severity);
    let icon: Option<&str> = match cv.alarm_severity {
        1 => Some(super::MINOR_ALARM_SVG),
        2 => Some(super::MAJOR_ALARM_SVG),
        3 => Some(super::INVALID_SVG),
        _ => None,
    };
    let display_value = cv.value_str.clone();
    let min = cv.control_low;
    let max = if (cv.control_high - cv.control_low).abs() < f64::EPSILON { cv.control_low + 100.0 } else { cv.control_high };
    let precision_step = 10f64.powi(-(cv.precision as i32).max(0));
    let step = config.metadata.as_ref()
        .and_then(|m| m.control.as_ref())
        .map(|c| c.min_step)
        .filter(|&s| s > 0.0)
        .unwrap_or(precision_step);
    render_slider_html(config, cv.raw_value, &display_value, &cv.units, min, max, step,
                        &format!("slider {}", alarm_class), icon, false, &super::build_tooltip(config, cv))
}

pub fn render_inner_disconnected(config: &WidgetConfig) -> Markup {
    render_slider_html(config, 0.0, "--", "", 0.0, 100.0, 0.1,
                        "slider alarm-disconnected", Some(super::OFFLINE_SVG), true, "")
}

fn render_slider_html(
    config: &WidgetConfig,
    current_value: f64,
    display_value: &str,
    units: &str,
    min: f64,
    max: f64,
    step: f64,
    _alarm_class: &str,
    icon: Option<&str>,
    disabled: bool,
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
            div class="slider-container" {
                input type="range"
                    class="widget-slider"
                    name="value"
                    min=(format!("{}", min))
                    max=(format!("{}", max))
                    step=(format!("{}", step))
                    value=(format!("{}", current_value))
                    disabled[disabled]
                    hx-post={"/api/widget/" (config.id) "/set"}
                    hx-trigger="change"
                    hx-target="next .slider-value";
                span class="slider-value" {
                    (display_value)
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

pub fn render_slider(widget: &WidgetConfig) -> Markup {
    html! {
        div style=[super::widget_container_style(widget)]
            data-widget-id=(widget.id)
            data-ch=(widget.channel_address())
            hx-sse=(format!("swap:{}", widget.id)) {
            (render_inner_disconnected(widget))
        }
    }
}
