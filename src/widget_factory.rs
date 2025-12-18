// Widget factory for dynamically creating widgets from protobuf configurations
use crate::generated::pv_service::*;
use crate::inputs::text_entry;
use crate::state::{TextEntryConfigState, AlarmStatus};

/// Dynamically render a widget from its protobuf configuration
pub fn render_widget(config: &WidgetConfig) -> Result<String, String> {
    match config.r#type() {
        WidgetType::TextEntry => render_text_entry(config),
        WidgetType::Gauge => render_gauge(config),
        WidgetType::Chart => render_chart(config),
        WidgetType::Button => render_button(config),
        WidgetType::Led => render_led(config),
        WidgetType::Slider => render_slider(config),
        WidgetType::Unspecified => Err("Widget type not specified".to_string()),
    }
}

/// Render a text entry widget from protobuf config
fn render_text_entry(config: &WidgetConfig) -> Result<String, String> {
    let text_config = match &config.config {
        Some(widget_config::Config::TextEntry(tc)) => tc,
        _ => return Err("Invalid config for text entry widget".to_string()),
    };
    
    // Convert protobuf config to internal state
    let state = TextEntryConfigState {
        pv_name: config.pv_name.clone(),
        has_units: text_config.show_units,
        units_text: String::new(), // Will be populated from PV metadata
        current_value: None,
        alarm_status: AlarmStatus::Normal,
        has_border: true,
        has_left_icon: false,
        has_right_icon: false,
    };
    
    // Generate widget HTML
    let mut html = text_entry::render(&state, &config.id);
    
    // Apply custom styling if provided
    if let Some(style) = &config.style {
        html = apply_common_style(html, style);
    }
    
    // Wrap with label if provided
    if !config.label.is_empty() {
        html = format!(
            r#"<div class="widget-container">
                <label style="color: #888; font-size: 12px; margin-bottom: 4px; display: block;">{}</label>
                {}
            </div>"#,
            config.label,
            html
        );
    }
    
    Ok(html)
}

/// Render a gauge widget (placeholder implementation)
fn render_gauge(config: &WidgetConfig) -> Result<String, String> {
    let gauge_config = match &config.config {
        Some(widget_config::Config::Gauge(gc)) => gc,
        _ => return Err("Invalid config for gauge widget".to_string()),
    };
    
    Ok(format!(
        r#"<div class="gauge-widget" data-widget-id="{}" data-pv="{}">
            <div style="text-align: center; padding: 20px; background: rgba(255,255,255,0.05); border-radius: 8px;">
                <div style="color: #888; font-size: 12px; margin-bottom: 8px;">GAUGE: {}</div>
                <div style="color: #00cc66; font-size: 24px; font-weight: bold;">0.00</div>
                <div style="color: #666; font-size: 10px; margin-top: 4px;">Range: {} - {}</div>
            </div>
        </div>"#,
        config.id,
        config.pv_name,
        config.label,
        gauge_config.min_value,
        gauge_config.max_value
    ))
}

/// Render a chart widget (placeholder)
fn render_chart(config: &WidgetConfig) -> Result<String, String> {
    Ok(format!(
        r#"<div class="chart-widget" data-widget-id="{}" data-pv="{}">
            <div style="background: rgba(0,0,0,0.3); padding: 20px; border-radius: 8px;">
                <div style="color: #fff; margin-bottom: 10px;">{}</div>
                <canvas id="chart-{}" width="400" height="200"></canvas>
            </div>
        </div>"#,
        config.id, config.pv_name, config.label, config.id
    ))
}

/// Render a button widget (placeholder)
fn render_button(config: &WidgetConfig) -> Result<String, String> {
    let button_config = match &config.config {
        Some(widget_config::Config::Button(bc)) => bc,
        _ => return Err("Invalid config for button widget".to_string()),
    };
    
    let label = if !button_config.label.is_empty() {
        &button_config.label
    } else {
        &config.label
    };
    
    Ok(format!(
        r#"<button 
            class="pv-button" 
            data-widget-id="{}" 
            data-pv="{}" 
            data-value="{}"
            style="padding: 12px 24px; background: #1e90ff; color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 14px;">
            {}
        </button>"#,
        config.id,
        config.pv_name,
        button_config.put_value,
        label
    ))
}

/// Render an LED indicator (placeholder)
fn render_led(config: &WidgetConfig) -> Result<String, String> {
    let led_config = match &config.config {
        Some(widget_config::Config::Led(lc)) => lc,
        _ => return Err("Invalid config for LED widget".to_string()),
    };
    
    Ok(format!(
        r#"<div class="led-widget" data-widget-id="{}" data-pv="{}">
            <div style="display: flex; align-items: center; gap: 10px;">
                <div style="width: {}px; height: {}px; border-radius: 50%; background: #00cc66; box-shadow: 0 0 10px #00cc66;"></div>
                {}
            </div>
        </div>"#,
        config.id,
        config.pv_name,
        led_config.size,
        led_config.size,
        if led_config.show_label {
            format!("<span style='color: #888;'>{}</span>", config.label)
        } else {
            String::new()
        }
    ))
}

/// Render a slider widget (placeholder)
fn render_slider(config: &WidgetConfig) -> Result<String, String> {
    let slider_config = match &config.config {
        Some(widget_config::Config::Slider(sc)) => sc,
        _ => return Err("Invalid config for slider widget".to_string()),
    };
    
    Ok(format!(
        r#"<div class="slider-widget" data-widget-id="{}" data-pv="{}">
            <label style="color: #888; font-size: 12px; display: block; margin-bottom: 8px;">{}</label>
            <input 
                type="range" 
                min="{}" 
                max="{}" 
                step="{}"
                style="width: 100%;">
            {}
        </div>"#,
        config.id,
        config.pv_name,
        config.label,
        slider_config.min_value,
        slider_config.max_value,
        slider_config.step,
        if slider_config.show_value {
            r#"<div style="color: #00cc66; font-size: 14px; margin-top: 4px; text-align: center;">0.0</div>"#
        } else {
            ""
        }
    ))
}

/// Apply common styling from protobuf config
fn apply_common_style(html: String, style: &CommonStyle) -> String {
    let mut styles = Vec::new();
    
    if style.width > 0 {
        styles.push(format!("width: {}px", style.width));
    }
    if style.height > 0 {
        styles.push(format!("height: {}px", style.height));
    }
    if let Some(padding) = &style.padding {
        styles.push(format!(
            "padding: {}px {}px {}px {}px",
            padding.top, padding.right, padding.bottom, padding.left
        ));
    }
    if let Some(margin) = &style.margin {
        styles.push(format!(
            "margin: {}px {}px {}px {}px",
            margin.top, margin.right, margin.bottom, margin.left
        ));
    }
    if !style.border_radius.is_empty() {
        styles.push(format!("border-radius: {}", style.border_radius));
    }
    if !style.box_shadow.is_empty() {
        styles.push(format!("box-shadow: {}", style.box_shadow));
    }
    
    if styles.is_empty() {
        return html;
    }
    
    // Wrap in a div with styling
    format!(
        r#"<div style="{}">{}</div>"#,
        styles.join("; "),
        html
    )
}

/// Render an entire page from protobuf configuration
pub fn render_page(page_config: &PageConfig) -> String {
    let mut widget_htmls = Vec::new();
    
    for widget in &page_config.widgets {
        match render_widget(widget) {
            Ok(html) => widget_htmls.push(html),
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to render widget {}: {}", widget.id, e).into());
            }
        }
    }
    
    // Apply page layout
    let layout_style = if let Some(layout) = &page_config.layout {
        match layout.r#type() {
            LayoutType::Grid => format!(
                "display: grid; grid-template-columns: repeat({}, 1fr); gap: {}px;",
                layout.columns, layout.gap
            ),
            LayoutType::FlexColumn => format!("display: flex; flex-direction: column; gap: {}px;", layout.gap),
            LayoutType::FlexRow => format!("display: flex; flex-direction: row; gap: {}px;", layout.gap),
            _ => String::new(),
        }
    } else {
        "display: flex; flex-direction: column; gap: 20px;".to_string()
    };
    
    let page_style = if let Some(style) = &page_config.style {
        format!(
            "background: {}; color: {}; font-family: {};",
            style.background,
            style.text_color,
            style.font_family
        )
    } else {
        String::new()
    };
    
    format!(
        r#"<div class="page-container" style="{} {}">
            <header style="margin-bottom: 30px;">
                <h1 style="margin: 0 0 8px 0; font-size: 28px;">{}</h1>
                <p style="margin: 0; opacity: 0.7; font-size: 14px;">{}</p>
            </header>
            <div class="widget-grid" style="{}">
                {}
            </div>
        </div>"#,
        page_style,
        "padding: 40px; min-height: 100vh;",
        page_config.title,
        page_config.description,
        layout_style,
        widget_htmls.join("\n")
    )
}
