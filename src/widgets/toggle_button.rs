use maud::{html, Markup};
use std::sync::Arc;
use futures::StreamExt;
use crate::channel::{ChannelContext, ChannelEvent, ChannelValue};
use crate::config::WidgetConfig;

pub struct ToggleButton {
    config: WidgetConfig,
}

impl ToggleButton {
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

pub(crate) fn render_inner_connected(config: &WidgetConfig, cv: &ChannelValue) -> Markup {
    let is_on = cv.raw_value > 0.5;
    let next_val = if is_on { "0" } else { "1" };
    render_toggle_html(config, is_on, next_val, false, &super::build_tooltip(config, cv))
}

pub(crate) fn render_inner_disconnected(config: &WidgetConfig) -> Markup {
    render_toggle_html(config, false, "0", true, "")
}

fn render_toggle_html(
    config: &WidgetConfig,
    is_on: bool,
    next_val: &str,
    disabled: bool,
    tooltip: &str,
) -> Markup {
    let btn_class = if is_on {
        "widget-button widget-toggle-btn widget-toggle-btn--on"
    } else {
        "widget-button widget-toggle-btn widget-toggle-btn--off"
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
                (config.label) " â€” " (state_label)
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
        div style=[super::widget_container_style(widget)]
            data-widget-id=(widget.id)
            data-ch=(widget.channel_address())
            hx-sse=(format!("swap:{}", widget.id)) {
            (render_inner_disconnected(widget))
        }
    }
}


#[cfg(test)]
#[path = "tests/toggle_button.rs"]
mod tests;
