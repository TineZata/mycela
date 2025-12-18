use crate::inputs::text_entry;
use crate::state::{AlarmStatus, TextEntryConfigState};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlInputElement, MessageEvent, WebSocket, Window};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

// ============================================================================
// WebSocket Protocol Types
// ============================================================================

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum WsRequest {
    #[serde(rename = "put")]
    Put { pv: String, value: f64 },
    #[serde(rename = "monitor")]
    Monitor { pv: String },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum WsResponse {
    #[serde(rename = "value")]
    Value {
        pv: String,
        value: f64,
        severity: Option<i32>,
        #[allow(dead_code)]
        status: Option<i32>,
        message: Option<String>,
    },
    #[serde(rename = "success")]
    Success { pv: String, message: String },
    #[serde(rename = "error")]
    Error { pv: String, message: String },
}

// ============================================================================
// Global State
// ============================================================================

thread_local! {
    static WEBSOCKET: RefCell<Option<WebSocket>> = RefCell::new(None);
    static IS_CONNECTED: RefCell<bool> = RefCell::new(false);
}

// ============================================================================
// Page Render
// ============================================================================

pub fn render() -> String {
    r#"
    <div class="pv-demo" style="width: 100%; min-height: 100vh; background: linear-gradient(135deg, #1e1e1e 0%, #2d2d2d 100%);">
        <!-- Header -->
        <header style="background: rgba(0, 0, 0, 0.3); backdrop-filter: blur(10px); padding: 30px; border-bottom: 1px solid #3e3e3e;">
            <h1 style="color: #fff; margin: 0 0 10px 0; font-size: 36px; font-weight: 300;">Control System Widgets</h1>
            <p style="color: #aaa; margin: 0; font-size: 16px;">EPICS PVAccess - PUT &amp; MONITOR Demo</p>
            <div id="connection-status" style="margin-top: 15px; padding: 8px 16px; border-radius: 4px; display: inline-block; background: #333; border: 1px solid #fa00fa;">
                <span style="color: #fa00fa; font-size: 14px;">● Connecting...</span>
            </div>
        </header>

        <!-- Main Content -->
        <div style="max-width: 800px; margin: 0 auto; padding: 40px 20px;">
            
            <!-- Demo Widget Section -->
            <section class="widget-section" style="margin-bottom: 40px;">
                <div class="section-header" style="margin-bottom: 20px;">
                    <h2 style="color: #fff; font-size: 24px; margin: 0 0 8px 0;">Motor Position Control</h2>
                    <p style="color: #888; margin: 0; font-size: 14px;">
                        Type a value and press <kbd style="background: #444; padding: 2px 6px; border-radius: 3px; font-size: 12px;">Enter</kbd> to PUT. 
                        Readback updates automatically via MONITOR.
                    </p>
                </div>
                
                <!-- Main Widget -->
                <div class="widget-card" style="background: rgba(255, 255, 255, 0.05); border: 1px solid #3e3e3e; border-radius: 12px; padding: 30px; margin-bottom: 30px;">
                    <h3 style="color: #00cc66; margin: 0 0 20px 0; font-size: 18px;">Position with Units</h3>
                    <div id="motor-widget"></div>
                </div>

                <!-- How It Works -->
                <div style="background: rgba(0, 102, 204, 0.1); border: 1px solid rgba(0, 102, 204, 0.3); border-radius: 8px; padding: 25px;">
                    <h3 style="color: #0066cc; font-size: 16px; margin: 0 0 15px 0;">How It Works</h3>
                    <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 20px; color: #aaa; font-size: 13px;">
                        <div>
                            <strong style="color: #fff;">1. PUT (Write)</strong>
                            <p style="margin: 5px 0 0 0;">Enter a value and press Enter. The bridge sends a <code style="background: #333; padding: 1px 4px; border-radius: 2px;">put_double()</code> to the EPICS server.</p>
                        </div>
                        <div>
                            <strong style="color: #fff;">2. MONITOR (Subscribe)</strong>
                            <p style="margin: 5px 0 0 0;">The bridge polls the PV every 500ms and pushes updates to all connected clients.</p>
                        </div>
                        <div>
                            <strong style="color: #fff;">3. Readback</strong>
                            <p style="margin: 5px 0 0 0;">The "Readback" display shows the actual value from the server, confirming your write succeeded.</p>
                        </div>
                        <div>
                            <strong style="color: #fff;">4. Alarms</strong>
                            <p style="margin: 5px 0 0 0;">Try values outside 5-100 to trigger alarm states. Border and icon change based on severity.</p>
                        </div>
                    </div>
                </div>
            </section>

            <!-- Alarm Test Section -->
            <section style="margin-bottom: 40px;">
                <h2 style="color: #fff; font-size: 20px; margin: 0 0 15px 0;">Test Alarm Limits</h2>
                <div style="display: grid; grid-template-columns: repeat(5, 1fr); gap: 10px;">
                    <button class="test-btn" data-value="3" style="background: #ff4444; color: white; border: none; padding: 12px; border-radius: 6px; cursor: pointer; font-size: 14px;">
                        3 mm<br><small>MAJOR LOW</small>
                    </button>
                    <button class="test-btn" data-value="8" style="background: #ffa500; color: white; border: none; padding: 12px; border-radius: 6px; cursor: pointer; font-size: 14px;">
                        8 mm<br><small>MINOR LOW</small>
                    </button>
                    <button class="test-btn" data-value="50" style="background: #00cc66; color: white; border: none; padding: 12px; border-radius: 6px; cursor: pointer; font-size: 14px;">
                        50 mm<br><small>NORMAL</small>
                    </button>
                    <button class="test-btn" data-value="95" style="background: #ffa500; color: white; border: none; padding: 12px; border-radius: 6px; cursor: pointer; font-size: 14px;">
                        95 mm<br><small>MINOR HIGH</small>
                    </button>
                    <button class="test-btn" data-value="105" style="background: #ff4444; color: white; border: none; padding: 12px; border-radius: 6px; cursor: pointer; font-size: 14px;">
                        105 mm<br><small>MAJOR HIGH</small>
                    </button>
                </div>
            </section>

            <!-- Console Log -->
            <section>
                <h2 style="color: #fff; font-size: 20px; margin: 0 0 15px 0;">Activity Log</h2>
                <div id="log-output" style="background: #111; border: 1px solid #333; border-radius: 6px; padding: 15px; height: 200px; overflow-y: auto; font-family: monospace; font-size: 12px; color: #888;">
                    <div style="color: #666;">Waiting for connection...</div>
                </div>
            </section>

        </div>
    </div>

    <style>
        .pv-demo * { box-sizing: border-box; }
        .test-btn:hover { opacity: 0.9; transform: scale(1.02); }
        .test-btn:active { transform: scale(0.98); }
        .test-btn small { display: block; margin-top: 4px; font-size: 10px; opacity: 0.8; }
        kbd { font-family: monospace; }
    </style>
    "#.to_string()
}

// ============================================================================
// Setup Handlers
// ============================================================================

pub fn setup_handlers(document: &Document, _window: &Window) -> Result<(), JsValue> {
    // Render initial widget (disconnected state)
    render_motor_widget(document)?;
    
    // Initialize WebSocket connection
    init_websocket(document)?;
    
    // Setup Enter key handler for input
    setup_input_handlers(document)?;
    
    // Setup test buttons
    setup_test_buttons(document)?;
    
    Ok(())
}

fn render_motor_widget(document: &Document) -> Result<(), JsValue> {
    if let Some(el) = document.get_element_by_id("motor-widget") {
        let config = TextEntryConfigState::new("wasm:test:motor:position")
            .with_units("mm");
        el.set_inner_html(&text_entry::render(&config, "motor"));
    }
    Ok(())
}

// ============================================================================
// WebSocket Management
// ============================================================================

fn init_websocket(document: &Document) -> Result<(), JsValue> {
    let ws = WebSocket::new("ws://127.0.0.1:8765")?;
    let doc_clone = document.clone();
    
    // onopen
    let onopen = {
        let doc = doc_clone.clone();
        Closure::wrap(Box::new(move |_| {
            IS_CONNECTED.with(|c| *c.borrow_mut() = true);
            update_connection_status(&doc, true);
            log_message(&doc, "Connected to EPICS WebSocket bridge", "success");
            
            // Start monitoring the PV
            if let Err(e) = send_request(WsRequest::Monitor { 
                pv: "wasm:test:motor:position".to_string() 
            }) {
                web_sys::console::error_1(&format!("Failed to start monitor: {:?}", e).into());
            }
        }) as Box<dyn FnMut(JsValue)>)
    };
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();
    
    // onclose
    let onclose = {
        let doc = doc_clone.clone();
        Closure::wrap(Box::new(move |_| {
            IS_CONNECTED.with(|c| *c.borrow_mut() = false);
            update_connection_status(&doc, false);
            log_message(&doc, "Disconnected from bridge. Retrying in 3s...", "error");
            update_widget_disconnected(&doc);

            // Try to reconnect after 3 seconds
            if let Some(window) = web_sys::window() {
                let doc_for_reconnect = doc.clone();
                let closure = Closure::wrap(Box::new(move || {
                    let _ = init_websocket(&doc_for_reconnect);
                }) as Box<dyn FnMut()>);
                let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    3000,
                );
                closure.forget();
            }
        }) as Box<dyn FnMut(JsValue)>)
    };
    ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();
    
    // onerror
    let onerror = {
        let doc = doc_clone.clone();
        Closure::wrap(Box::new(move |_: JsValue| {
            log_message(&doc, "WebSocket error", "error");
        }) as Box<dyn FnMut(JsValue)>)
    };
    ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    onerror.forget();
    
    // onmessage
    let onmessage = {
        let doc = doc_clone;
        Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                let data: String = txt.into();
                if let Ok(response) = serde_json::from_str::<WsResponse>(&data) {
                    handle_response(&doc, response);
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>)
    };
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();
    
    WEBSOCKET.with(|ws_cell| {
        *ws_cell.borrow_mut() = Some(ws);
    });
    
    Ok(())
}

fn send_request(request: WsRequest) -> Result<(), JsValue> {
    WEBSOCKET.with(|ws_cell| {
        if let Some(ws) = ws_cell.borrow().as_ref() {
            let json = serde_json::to_string(&request)
                .map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))?;
            ws.send_with_str(&json)
        } else {
            Err(JsValue::from_str("Not connected"))
        }
    })
}

// ============================================================================
// Response Handling
// ============================================================================

fn handle_response(document: &Document, response: WsResponse) {
    match response {
        WsResponse::Value { pv, value, severity, message, .. } => {
            // Update the widget UI
            update_widget_value(document, &pv, value, severity);
            
            let severity_str = match severity {
                Some(0) => "NORMAL",
                Some(1) => "MINOR",
                Some(2) => "MAJOR",
                Some(3) => "INVALID",
                _ => "NORMAL",
            };
            
            let msg = format!("← {} = {:.2} [{}] {}", 
                pv, value, severity_str, message.unwrap_or_default());
            log_message(document, &msg, if severity.unwrap_or(0) > 0 { "warning" } else { "info" });
        }
        WsResponse::Success { pv, message } => {
            log_message(document, &format!("✓ PUT {}: {}", pv, message), "success");
        }
        WsResponse::Error { pv, message } => {
            log_message(document, &format!("✗ {} error: {}", pv, message), "error");
        }
    }
}

fn update_widget_value(document: &Document, _pv: &str, value: f64, severity: Option<i32>) {
    // Update input field with current PV value (only if not focused)
    if let Some(input) = document.get_element_by_id("input-motor") {
        if let Ok(input_el) = input.clone().dyn_into::<HtmlInputElement>() {
            // Only update if the input is not currently focused (user is not typing)
            if document.active_element() != Some(input.clone()) {
                input_el.set_value(&format!("{:.2}", value));
            }
        }
        
        // Remove disconnection icon if present (first connection)
        if let Some(parent) = input.parent_element() {
            // Find and remove the icon img element
            if let Ok(imgs) = parent.query_selector_all("img[alt='offline']") {
                for i in 0..imgs.length() {
                    if let Some(img) = imgs.get(i) {
                        let _ = img.parent_element().and_then(|p| p.remove_child(&img).ok());
                    }
                }
            }
        }
    }
    
    // Update readback display
    if let Some(readback_el) = document.get_element_by_id("readback-motor") {
        let alarm_status = AlarmStatus::from_severity(severity);
        let color = match alarm_status {
            AlarmStatus::Major => "#ff0000",
            AlarmStatus::Minor => "#ffa500",
            AlarmStatus::Invalid => "#999999",
            AlarmStatus::NotConnected => "#fa00fa",
            AlarmStatus::Normal => "#00cc66",
        };
        readback_el.set_inner_html(&format!("{:.2} mm", value));
        let _ = readback_el.set_attribute("style", &format!(
            "color: {}; font-size: 16px; font-weight: bold; font-family: monospace;", color
        ));
    }
    
    // Update status text
    if let Some(status_el) = document.get_element_by_id("status-motor") {
        let (status_text, color) = match severity {
            Some(0) => ("NORMAL", "#00cc66"),
            Some(1) => ("MINOR ALARM", "#ffa500"),
            Some(2) => ("MAJOR ALARM", "#ff0000"),
            Some(3) => ("INVALID", "#999999"),
            _ => ("NORMAL", "#00cc66"),
        };
        status_el.set_inner_html(status_text);
        let _ = status_el.set_attribute("style", &format!(
            "color: {}; font-size: 10px; font-family: monospace;", color
        ));
    }
    
    // Update border color on the widget container
    update_widget_border(document, severity);
}

fn update_widget_border(document: &Document, severity: Option<i32>) {
    if let Some(input_el) = document.get_element_by_id("input-motor") {
        let (border_color, border_width) = match severity {
            Some(1) => ("#ffa500", "2px"),
            Some(2) => ("#ff0000", "2px"),
            Some(3) => ("#999999", "2px"),
            _ => ("#1e90ff", "1px"),
        };
        
        // Get current style and update border
        if let Some(style) = input_el.get_attribute("style") {
            let new_style = style
                .split(';')
                .filter(|s| !s.trim().starts_with("border:") && !s.trim().starts_with("border-right:"))
                .collect::<Vec<_>>()
                .join(";");
            let _ = input_el.set_attribute("style", &format!(
                "{}; border: {} solid {}; border-right: none;", new_style, border_width, border_color
            ));
        }
    }
}

fn update_widget_disconnected(document: &Document) {
    if let Some(readback_el) = document.get_element_by_id("readback-motor") {
        readback_el.set_inner_html("--- mm");
        let _ = readback_el.set_attribute("style", 
            "color: #fa00fa; font-size: 16px; font-weight: bold; font-family: monospace;");
    }
    
    if let Some(status_el) = document.get_element_by_id("status-motor") {
        status_el.set_inner_html("DISCONNECTED");
        let _ = status_el.set_attribute("style", 
            "color: #fa00fa; font-size: 10px; font-family: monospace;");
    }
}

fn update_connection_status(document: &Document, connected: bool) {
    if let Some(el) = document.get_element_by_id("connection-status") {
        if connected {
            el.set_inner_html("<span style='color: #00cc66; font-size: 14px;'>● Connected</span>");
            let _ = el.set_attribute("style", 
                "margin-top: 15px; padding: 8px 16px; border-radius: 4px; display: inline-block; background: #1a3320; border: 1px solid #00cc66;");
        } else {
            el.set_inner_html("<span style='color: #fa00fa; font-size: 14px;'>● Disconnected</span>");
            let _ = el.set_attribute("style", 
                "margin-top: 15px; padding: 8px 16px; border-radius: 4px; display: inline-block; background: #333; border: 1px solid #fa00fa;");
        }
    }
}

// ============================================================================
// Input Handlers
// ============================================================================

fn setup_input_handlers(document: &Document) -> Result<(), JsValue> {
    if let Some(input) = document.get_element_by_id("input-motor") {
        if let Ok(input_el) = input.dyn_into::<HtmlInputElement>() {
            let input_clone = input_el.clone();
            let doc_clone = document.clone();
            
            let handler = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
                if e.key() == "Enter" {
                    let value_str = input_clone.value();
                    if let Ok(value) = value_str.parse::<f64>() {
                        let pv = "wasm:test:motor:position".to_string();
                        log_message(&doc_clone, &format!("→ PUT {} = {}", pv, value), "info");
                        
                        if let Err(e) = send_request(WsRequest::Put { pv, value }) {
                            log_message(&doc_clone, &format!("Failed to send: {:?}", e), "error");
                        }
                        
                        // Clear the input after sending
                        input_clone.set_value("");
                    } else {
                        log_message(&doc_clone, &format!("Invalid number: {}", value_str), "error");
                    }
                }
            }) as Box<dyn FnMut(_)>);
            
            input_el.add_event_listener_with_callback("keydown", handler.as_ref().unchecked_ref())?;
            handler.forget();
        }
    }
    Ok(())
}

fn setup_test_buttons(document: &Document) -> Result<(), JsValue> {
    let buttons = document.query_selector_all(".test-btn")?;
    
    for i in 0..buttons.length() {
        if let Some(node) = buttons.get(i) {
            if let Ok(btn) = node.dyn_into::<Element>() {
                let doc_clone = document.clone();
                let btn_clone = btn.clone();
                
                let handler = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
                    if let Some(value_str) = btn_clone.get_attribute("data-value") {
                        if let Ok(value) = value_str.parse::<f64>() {
                            let pv = "wasm:test:motor:position".to_string();
                            log_message(&doc_clone, &format!("→ PUT {} = {} (test button)", pv, value), "info");
                            
                            if let Err(e) = send_request(WsRequest::Put { pv, value }) {
                                log_message(&doc_clone, &format!("Failed to send: {:?}", e), "error");
                            }
                        }
                    }
                }) as Box<dyn FnMut(_)>);
                
                btn.add_event_listener_with_callback("click", handler.as_ref().unchecked_ref())?;
                handler.forget();
            }
        }
    }
    
    Ok(())
}

// ============================================================================
// Logging
// ============================================================================

fn log_message(document: &Document, message: &str, level: &str) {
    if let Some(log_el) = document.get_element_by_id("log-output") {
        let color = match level {
            "success" => "#00cc66",
            "error" => "#ff4444",
            "warning" => "#ffa500",
            _ => "#aaa",
        };
        
        let timestamp = js_sys::Date::new_0();
        let time_str = format!("{:02}:{:02}:{:02}", 
            timestamp.get_hours(),
            timestamp.get_minutes(),
            timestamp.get_seconds()
        );
        
        let new_line = format!(
            "<div style='margin-bottom: 4px;'><span style='color: #666;'>[{}]</span> <span style='color: {};'>{}</span></div>",
            time_str, color, message
        );
        
        let current = log_el.inner_html();
        log_el.set_inner_html(&format!("{}{}", current, new_line));
        
        // Auto-scroll to bottom
        if let Some(html_el) = log_el.dyn_ref::<web_sys::HtmlElement>() {
            html_el.set_scroll_top(html_el.scroll_height());
        }
    }
    
    // Also log to browser console
    web_sys::console::log_1(&message.into());
}
