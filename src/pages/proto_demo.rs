// Example page that uses protobuf-defined widgets
use crate::generated::pv_service::*;
use crate::widget_factory;
use wasm_bindgen::prelude::*;

/// Create a sample page configuration
pub fn create_sample_page() -> PageConfig {
    PageConfig {
        id: "motor_control".to_string(),
        title: "Motor Control Dashboard".to_string(),
        description: "Dynamically generated from protobuf configuration".to_string(),
        widgets: vec![
            // Text entry for position setpoint
            WidgetConfig {
                id: "motor_position_sp".to_string(),
                pv_name: "demo:motor:position".to_string(),
                r#type: WidgetType::TextEntry as i32,
                config: Some(widget_config::Config::TextEntry(TextEntryConfig {
                    show_units: true,
                    show_readback: true,
                    precision: 2,
                    min_value: -100.0,
                    max_value: 100.0,
                    placeholder: "Enter position...".to_string(),
                    entry_style: Some(TextEntryStyle {
                        background_color: "#e6f3ff".to_string(),
                        text_color: "#333".to_string(),
                        border_style: "solid".to_string(),
                        font_size: 15,
                        font_family: "monospace".to_string(),
                    }),
                    readback_style: Some(ReadbackStyle {
                        background_color: "#f0f0f0".to_string(),
                        text_color: "#00cc66".to_string(),
                        show_alarm_border: true,
                        alarm_viz: AlarmVisualization::IconAndBorder as i32,
                    }),
                    auto_submit: false,
                    debounce_ms: 0.0,
                })),
                style: Some(CommonStyle {
                    width: 0,
                    height: 0,
                    padding: Some(Padding {
                        top: 10,
                        right: 0,
                        bottom: 10,
                        left: 0,
                    }),
                    margin: None,
                    border_radius: String::new(),
                    box_shadow: String::new(),
                }),
                label: "Motor Position".to_string(),
                description: "Setpoint for motor position in mm".to_string(),
                layout: None,
            },
            // Text entry for velocity
            WidgetConfig {
                id: "motor_velocity_sp".to_string(),
                pv_name: "demo:motor:velocity".to_string(),
                r#type: WidgetType::TextEntry as i32,
                config: Some(widget_config::Config::TextEntry(TextEntryConfig {
                    show_units: true,
                    show_readback: true,
                    precision: 1,
                    min_value: 0.0,
                    max_value: 50.0,
                    placeholder: "Enter velocity...".to_string(),
                    entry_style: Some(TextEntryStyle {
                        background_color: "#fff5e6".to_string(),
                        text_color: "#333".to_string(),
                        border_style: "solid".to_string(),
                        font_size: 15,
                        font_family: "monospace".to_string(),
                    }),
                    readback_style: Some(ReadbackStyle {
                        background_color: "#f0f0f0".to_string(),
                        text_color: "#ff8800".to_string(),
                        show_alarm_border: true,
                        alarm_viz: AlarmVisualization::BorderOnly as i32,
                    }),
                    auto_submit: false,
                    debounce_ms: 0.0,
                })),
                style: None,
                label: "Motor Velocity".to_string(),
                description: "Target velocity in mm/s".to_string(),
                layout: None,
            },
            // Gauge for current
            WidgetConfig {
                id: "motor_current_gauge".to_string(),
                pv_name: "motor:current:rb".to_string(),
                r#type: WidgetType::Gauge as i32,
                config: Some(widget_config::Config::Gauge(GaugeConfig {
                    min_value: 0.0,
                    max_value: 10.0,
                    num_ticks: 10,
                    show_needle: true,
                    show_value_text: true,
                    ranges: vec![
                        GaugeRange {
                            start: 0.0,
                            end: 5.0,
                            color: "#00cc66".to_string(),
                            label: "Normal".to_string(),
                        },
                        GaugeRange {
                            start: 5.0,
                            end: 8.0,
                            color: "#ffa500".to_string(),
                            label: "Warning".to_string(),
                        },
                        GaugeRange {
                            start: 8.0,
                            end: 10.0,
                            color: "#ff0000".to_string(),
                            label: "Danger".to_string(),
                        },
                    ],
                })),
                style: None,
                label: "Motor Current (A)".to_string(),
                description: String::new(),
                layout: None,
            },
            // LED for motor status
            WidgetConfig {
                id: "motor_enabled_led".to_string(),
                pv_name: "motor:enabled".to_string(),
                r#type: WidgetType::Led as i32,
                config: Some(widget_config::Config::Led(LedConfig {
                    states: vec![
                        LedState {
                            value: 0.0,
                            color: "#666666".to_string(),
                            label: "Disabled".to_string(),
                        },
                        LedState {
                            value: 1.0,
                            color: "#00cc66".to_string(),
                            label: "Enabled".to_string(),
                        },
                    ],
                    size: 24,
                    show_label: true,
                })),
                style: None,
                label: "Motor Status".to_string(),
                description: String::new(),
                layout: None,
            },
            // Button to enable/disable
            WidgetConfig {
                id: "motor_enable_btn".to_string(),
                pv_name: "motor:enable:cmd".to_string(),
                r#type: WidgetType::Button as i32,
                config: Some(widget_config::Config::Button(ButtonConfig {
                    put_value: 1.0,
                    label: "Enable Motor".to_string(),
                    button_style: Some(ButtonStyle {
                        background_color: "#1e90ff".to_string(),
                        text_color: "#ffffff".to_string(),
                        hover_color: "#1c7ed6".to_string(),
                        width: 150,
                        height: 40,
                    }),
                    confirm_action: true,
                })),
                style: None,
                label: String::new(),
                description: String::new(),
                layout: None,
            },
        ],
        layout: Some(PageLayout {
            r#type: LayoutType::Grid as i32,
            columns: 2,
            gap: 20,
        }),
        style: Some(PageStyle {
            background: "linear-gradient(135deg, #1e1e1e 0%, #2d2d2d 100%)".to_string(),
            text_color: "#ffffff".to_string(),
            font_family: "system-ui, -apple-system, sans-serif".to_string(),
        }),
    }
}

/// Render the demo page
pub fn render() -> String {
    let page_config = create_sample_page();
    widget_factory::render_page(&page_config)
}

/// Setup input event handlers for PV text entry widgets
fn setup_pv_input_handlers(document: &web_sys::Document) -> Result<(), JsValue> {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::spawn_local;
    use crate::grpc_client;
    
    // Find all input elements with data-pv attribute
    let inputs = document.query_selector_all("input.pv-input")?;
    web_sys::console::log_1(&format!("[SETUP] Found {} input elements with pv-input class", inputs.length()).into());
    
    for i in 0..inputs.length() {
        if let Some(input) = inputs.get(i) {
            if let Ok(input_element) = input.dyn_into::<web_sys::HtmlInputElement>() {
                let pv_name = input_element.get_attribute("data-pv");
                
                if let Some(pv_name) = pv_name {
                    web_sys::console::log_1(&format!("[SETUP] Attaching handler to input for PV: {}", pv_name).into());
                    
                    // Clone for the closure
                    let pv_name_clone = pv_name.clone();
                    let input_clone = input_element.clone();
                    
                    // Handle Enter key press
                    let keypress_closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                        if event.key() == "Enter" {
                            let value_str = input_clone.value();
                            
                            // Parse the input value
                            if let Ok(value) = value_str.parse::<f64>() {
                                web_sys::console::log_1(&format!("Sending PV write: {} = {}", pv_name_clone, value).into());
                                
                                let pv_name_for_async = pv_name_clone.clone();
                                spawn_local(async move {
                                    match grpc_client::put_pv_value(&pv_name_for_async, value).await {
                                        Ok(_) => {
                                            web_sys::console::log_1(&format!("Successfully wrote {} = {}", pv_name_for_async, value).into());
                                        }
                                        Err(e) => {
                                            web_sys::console::error_1(&format!("Error writing PV: {}", e).into());
                                        }
                                    }
                                });
                            } else {
                                web_sys::console::error_1(&format!("Invalid number: {}", value_str).into());
                            }
                        }
                    }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
                    
                    input_element.add_event_listener_with_callback(
                        "keydown",
                        keypress_closure.as_ref().unchecked_ref()
                    )?;
                    keypress_closure.forget();
                    
                    web_sys::console::log_1(&format!("[SETUP] Handler attached successfully for {}", pv_name).into());
                }
            }
        }
    }
    
    Ok(())
}

/// Setup event handlers and start live PV monitoring
pub fn setup_handlers(
    document: &web_sys::Document,
    _window: &web_sys::Window,
) -> Result<(), JsValue> {
    web_sys::console::log_1(&"Proto demo page handlers setup".into());
    
    // Debug: Check all inputs in the page
    if let Ok(all_inputs) = document.query_selector_all("input") {
        web_sys::console::log_1(&format!("[DEBUG] Total input elements: {}", all_inputs.length()).into());
        for i in 0..all_inputs.length() {
            if let Some(input) = all_inputs.get(i) {
                if let Ok(elem) = input.dyn_into::<web_sys::HtmlInputElement>() {
                    let class_name = elem.class_name();
                    let data_pv = elem.get_attribute("data-pv").unwrap_or_else(|| "none".to_string());
                    web_sys::console::log_1(&format!("[DEBUG] Input {}: class='{}', data-pv='{}'", i, class_name, data_pv).into());
                }
            }
        }
    }
    
    // Set up input event handlers for all PV inputs
    setup_pv_input_handlers(document)?;
    
    // Start monitoring the demo PV
    use wasm_bindgen_futures::spawn_local;
    use crate::grpc_client;
    
    spawn_local(async move {
        // web_sys::console::log_1(&"Fetching demo PV value...".into());
        
        match grpc_client::get_pv_value("demo:motor:position").await {
            Ok(pv_value) => {
                let value = match pv_value.value {
                    Some(crate::generated::pv_service::pv_value::Value::DoubleValue(v)) => v,
                    Some(crate::generated::pv_service::pv_value::Value::Int32Value(v)) => v as f64,
                    _ => 0.0,
                };
                
                // web_sys::console::log_1(&format!("PV Value: {}", value).into());
                
                // Display the value in the UI
                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        if let Some(app) = document.get_element_by_id("app") {
                            let status_html = format!(
                                r#"<div style="position: fixed; top: 10px; right: 10px; 
                                   background: rgba(0, 204, 102, 0.2); 
                                   border: 2px solid #00cc66; 
                                   padding: 10px 20px; 
                                   border-radius: 8px; 
                                   color: #00cc66; 
                                   font-family: monospace;">
                                   <strong>Live PV:</strong> demo:motor:position = {:.2}
                                </div>"#,
                                value
                            );
                            
                            let div = document.create_element("div").ok();
                            if let Some(div) = div {
                                div.set_inner_html(&status_html);
                                let _ = app.append_child(&div);
                            }
                        }
                    }
                }
                
                // Set up periodic updates that also update the UI
                let callback = wasm_bindgen::closure::Closure::wrap(Box::new(move |value: f64, pv_name: String| {
                    // web_sys::console::log_1(&format!("PV Update: {} = {:.2}", pv_name, value).into());
                    
                    // Update the live display in the top right corner
                    if let Some(window) = web_sys::window() {
                        if let Some(document) = window.document() {
                            if let Some(live_display) = document.query_selector("div[style*='position: fixed']").ok().flatten() {
                                let updated_html = format!(
                                    r#"<strong>Live PV:</strong> demo:motor:position = {:.2}"#,
                                    value
                                );
                                live_display.set_inner_html(&updated_html);
                            }
                        }
                    }
                }) as Box<dyn FnMut(f64, String)>);
                
                let js_callback: js_sys::Function = callback.as_ref().clone().into();
                grpc_client::start_pv_monitoring("demo:motor:position".to_string(), js_callback);
                callback.forget(); // Keep the closure alive
            }
            Err(e) => {
                web_sys::console::error_1(&format!("Error fetching PV: {}", e).into());
                
                // Show error in UI
                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        if let Some(app) = document.get_element_by_id("app") {
                            let error_html = format!(
                                r#"<div style="position: fixed; top: 10px; right: 10px; 
                                   background: rgba(255, 100, 100, 0.2); 
                                   border: 2px solid #ff6464; 
                                   padding: 10px 20px; 
                                   border-radius: 8px; 
                                   color: #ff6464; 
                                   font-family: monospace;">
                                   <strong>gRPC Error:</strong> {}<br>
                                   <small>Is grpc-server running on :50051?</small>
                                </div>"#,
                                e
                            );
                            
                            let div = document.create_element("div").ok();
                            if let Some(div) = div {
                                div.set_inner_html(&error_html);
                                let _ = app.append_child(&div);
                            }
                        }
                    }
                }
            }
        }
    });
    
    Ok(())
}
