mod state;
mod pages;
pub mod inputs;

#[cfg(feature = "proto")]
pub mod generated {
    pub mod pv_service {
        include!("generated/epics.pv.rs");
    }
}

#[cfg(feature = "proto")]
mod widget_factory;

#[cfg(feature = "proto")]
pub mod grpc_client;

use wasm_bindgen::prelude::*;

// Module exports
pub use state::*;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    // Set up panic hook for better error messages
    console_error_panic_hook::set_once();
    
    web_sys::console::log_1(&"WASM module initialized".into());
    
    Ok(())
}

#[cfg(feature = "proto")]
#[wasm_bindgen]
pub fn init_proto_demo() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
    let document = window.document().ok_or_else(|| JsValue::from_str("No document"))?;
    
    // Get the root element
    let root = document
        .get_element_by_id("app")
        .ok_or_else(|| JsValue::from_str("No app element"))?;
    
    // Render the proto demo page
    let html = pages::proto_demo::render();
    root.set_inner_html(&html);
    
    // Setup event handlers
    pages::proto_demo::setup_handlers(&document, &window)?;
    
    web_sys::console::log_1(&"Proto demo initialized successfully".into());
    
    Ok(())
}
