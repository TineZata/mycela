// gRPC-Web client for WASM
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::console;

#[cfg(feature = "proto")]
use crate::generated::pv_service::*;

const GRPC_SERVER_URL: &str = "http://127.0.0.1:50051";

/// Fetch a PV value from the gRPC server
pub async fn get_pv_value(pv_name: &str) -> Result<PvValue, String> {
    // web_sys::console::log_1(&format!("[CLIENT] get_pv_value called for: {}", pv_name).into());
    
    // Create the request body
    let request = GetPvRequest {
        name: pv_name.to_string(),
        timeout: 5.0,
    };
    
    // Serialize to protobuf
    use prost::Message;
    let mut buf = Vec::new();
    request.encode(&mut buf).map_err(|e| format!("Encode error: {}", e))?;
    
    // Add 5-byte gRPC-Web frame header: [compression-flag: 1 byte][message-length: 4 bytes big-endian]
    let message_len = buf.len() as u32;
    let mut frame = vec![0u8]; // No compression
    frame.extend_from_slice(&message_len.to_be_bytes());
    frame.extend_from_slice(&buf);
    
    // Make HTTP POST request to gRPC-Web endpoint
    let url = format!("{}/epics.pv.PVService/GetPV", GRPC_SERVER_URL);
    // web_sys::console::log_1(&format!("[CLIENT] Sending GET request to: {} (frame size: {})", url, frame.len()).into());
    
    let mut opts = web_sys::RequestInit::new();
    opts.method("POST");
    opts.mode(web_sys::RequestMode::Cors);
    
    // Set headers
    let headers = web_sys::Headers::new().map_err(|e| format!("Headers error: {:?}", e))?;
    headers.set("Content-Type", "application/grpc-web+proto")
        .map_err(|e| format!("Set header error: {:?}", e))?;
    headers.set("X-Grpc-Web", "1")
        .map_err(|e| format!("Set header error: {:?}", e))?;
    opts.headers(&headers);
    
    // Set body with frame
    let uint8_array = js_sys::Uint8Array::from(&frame[..]);
    opts.body(Some(&uint8_array));
    
    let request = web_sys::Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("Request error: {:?}", e))?;
    
    let window = web_sys::window().ok_or("No window")?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch error: {:?}", e))?;
    
    let resp: web_sys::Response = resp_value.dyn_into()
        .map_err(|e| format!("Response cast error: {:?}", e))?;
    
    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }
    
    // Get response body
    let array_buffer = wasm_bindgen_futures::JsFuture::from(
        resp.array_buffer()
            .map_err(|e| format!("Array buffer error: {:?}", e))?
    )
    .await
    .map_err(|e| format!("Array buffer future error: {:?}", e))?;
    
    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
    let mut response_bytes = vec![0; uint8_array.length() as usize];
    uint8_array.copy_to(&mut response_bytes);
    
    // web_sys::console::log_1(&format!("[CLIENT] Response bytes length: {}", response_bytes.len()).into());
    // if response_bytes.len() > 0 {
    //     let preview = &response_bytes[..std::cmp::min(20, response_bytes.len())];
    //     web_sys::console::log_1(&format!("[CLIENT] First 20 bytes: {:?}", preview).into());
    //     
    //     // Log as hex for easier debugging
    //     let hex: String = preview.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ");
    //     web_sys::console::log_1(&format!("[CLIENT] First 20 bytes (hex): {}", hex).into());
    // }
    
    // gRPC-Web format: [compression-flag: 1 byte][message-length: 4 bytes big-endian][message data]
    // Parse the frame
    if response_bytes.len() < 5 {
        return Err(format!("Response too short: {} bytes", response_bytes.len()));
    }
    
    let compression_flag = response_bytes[0];
    let message_len = u32::from_be_bytes([
        response_bytes[1],
        response_bytes[2],
        response_bytes[3],
        response_bytes[4],
    ]) as usize;
    
    // web_sys::console::log_1(&format!("[CLIENT] Compression flag: {}, Message length: {}", compression_flag, message_len).into());
    
    if compression_flag != 0 {
        return Err(format!("Compressed responses not supported (flag={})", compression_flag));
    }
    
    if response_bytes.len() < 5 + message_len {
        return Err(format!("Response truncated: expected {} bytes, got {}", 5 + message_len, response_bytes.len()));
    }
    
    let response_data = &response_bytes[5..5 + message_len];
    // web_sys::console::log_1(&format!("[CLIENT] Protobuf data length: {}", response_data.len()).into());
    
    // Decode protobuf response
    let pv_value = PvValue::decode(response_data)
        .map_err(|e| format!("Decode error: {}", e))?;
    
    Ok(pv_value)
}

/// Put a value to a PV
pub async fn put_pv_value(pv_name: &str, value: f64) -> Result<(), String> {
    web_sys::console::log_1(&format!("[PUT] Sending: {} = {}", pv_name, value).into());
    
    let request = PutPvRequest {
        name: pv_name.to_string(),
        value: Some(put_pv_request::Value::DoubleValue(value)),
        timeout: 5.0,
    };
    
    use prost::Message;
    let mut buf = Vec::new();
    request.encode(&mut buf).map_err(|e| format!("Encode error: {}", e))?;
    
    // Add 5-byte gRPC-Web frame header: [compression-flag: 1 byte][message-length: 4 bytes big-endian]
    let message_len = buf.len() as u32;
    let mut frame = vec![0u8]; // No compression
    frame.extend_from_slice(&message_len.to_be_bytes());
    frame.extend_from_slice(&buf);
    
    let url = format!("{}/epics.pv.PVService/PutPV", GRPC_SERVER_URL);
    web_sys::console::log_1(&format!("[CLIENT] Sending PUT request to: {} (frame size: {})", url, frame.len()).into());
    
    let mut opts = web_sys::RequestInit::new();
    opts.method("POST");
    opts.mode(web_sys::RequestMode::Cors);
    
    let headers = web_sys::Headers::new().map_err(|e| format!("Headers error: {:?}", e))?;
    headers.set("Content-Type", "application/grpc-web+proto")
        .map_err(|e| format!("Set header error: {:?}", e))?;
    headers.set("X-Grpc-Web", "1")
        .map_err(|e| format!("Set header error: {:?}", e))?;
    opts.headers(&headers);
    
    let uint8_array = js_sys::Uint8Array::from(&frame[..]);
    opts.body(Some(&uint8_array));
    
    let request = web_sys::Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("Request error: {:?}", e))?;
    
    let window = web_sys::window().ok_or("No window")?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch error: {:?}", e))?;
    
    let resp: web_sys::Response = resp_value.dyn_into()
        .map_err(|e| format!("Response cast error: {:?}", e))?;
    
    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }
    
    Ok(())
}

/// Subscribe to PV updates (using polling for now)
pub fn start_pv_monitoring(pv_name: String, callback: js_sys::Function) {
    spawn_local(async move {
        loop {
            match get_pv_value(&pv_name).await {
                Ok(pv_value) => {
                    // Call the JavaScript callback with the value
                    if let Some(value) = &pv_value.value {
                        let val = match value {
                            pv_value::Value::DoubleValue(v) => *v,
                            pv_value::Value::Int32Value(v) => *v as f64,
                            _ => 0.0,
                        };
                        
                        let this = JsValue::NULL;
                        let _ = callback.call2(&this, &JsValue::from_f64(val), &JsValue::from_str(&pv_name));
                    }
                }
                Err(e) => {
                    console::error_1(&format!("Error fetching PV {}: {}", pv_name, e).into());
                }
            }
            
            // Wait 1 second before next poll
            wasm_bindgen_futures::JsFuture::from(
                js_sys::Promise::new(&mut |resolve, _| {
                    web_sys::window()
                        .unwrap()
                        .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 1000)
                        .unwrap();
                })
            )
            .await
            .ok();
        }
    });
}

/// Fetch page configuration from the gRPC server
pub async fn get_page_config(_page_id: &str) -> Result<PageConfig, String> {
    // PageConfigRequest not available in WASM build
    Err("get_page_config not implemented for WASM".to_string())
}

// WASM bindings for JavaScript
#[wasm_bindgen]
pub async fn grpc_get_pv(pv_name: String) -> Result<JsValue, JsValue> {
    let pv_value = get_pv_value(&pv_name)
        .await
        .map_err(|e| JsValue::from_str(&e))?;
    
    // Convert to JSON for JavaScript
    let value = match pv_value.value {
        Some(pv_value::Value::DoubleValue(v)) => v,
        Some(pv_value::Value::Int32Value(v)) => v as f64,
        _ => 0.0,
    };
    
    Ok(JsValue::from_f64(value))
}

#[wasm_bindgen]
pub async fn grpc_put_pv(pv_name: String, value: f64) -> Result<(), JsValue> {
    put_pv_value(&pv_name, value)
        .await
        .map_err(|e| JsValue::from_str(&e))
}

#[wasm_bindgen]
pub fn grpc_monitor_pv(pv_name: String, callback: js_sys::Function) {
    start_pv_monitoring(pv_name, callback);
}
