use pvxs_sys::Context;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum Request {
    #[serde(rename = "get")]
    Get { pv: String },
    #[serde(rename = "put")]
    Put { pv: String, value: f64 },
    #[serde(rename = "monitor")]
    Monitor { pv: String },
    #[serde(rename = "unmonitor")]
    Unmonitor { pv: String },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum Response {
    #[serde(rename = "value")]
    Value {
        pv: String,
        value: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        severity: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    #[serde(rename = "error")]
    Error {
        pv: String,
        message: String,
    },
    #[serde(rename = "success")]
    Success {
        pv: String,
        message: String,
    },
}

/// Per-client state: which PVs this client is monitoring
struct ClientState {
    monitored_pvs: HashSet<String>,
    tx: mpsc::UnboundedSender<Response>,
}

async fn handle_client(stream: TcpStream, ctx: Arc<Mutex<Context>>) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("WebSocket handshake failed: {}", e);
            return;
        }
    };

    println!("New WebSocket connection established");
    let (write, mut read) = ws_stream.split();

    // Channel for sending responses to the client
    let (tx, mut rx) = mpsc::unbounded_channel::<Response>();
    
    // Shared client state
    let client_state = Arc::new(Mutex::new(ClientState {
        monitored_pvs: HashSet::new(),
        tx: tx.clone(),
    }));

    // Task to handle sending messages to the WebSocket
    let write_task = {
        let mut write = write;
        async move {
            while let Some(response) = rx.recv().await {
                let json = serde_json::to_string(&response).unwrap();
                if let Err(e) = write.send(Message::Text(json)).await {
                    eprintln!("Failed to send response: {}", e);
                    break;
                }
            }
        }
    };

    // Task to poll monitored PVs periodically
    let poll_task = {
        let ctx = Arc::clone(&ctx);
        let client_state = Arc::clone(&client_state);
        async move {
            let mut poll_interval = interval(Duration::from_millis(200));
            loop {
                poll_interval.tick().await;
                
                let (pvs, tx) = {
                    let state = client_state.lock().unwrap();
                    (state.monitored_pvs.clone(), state.tx.clone())
                };
                
                if pvs.is_empty() {
                    continue;
                }

                // Spawn a blocking task to handle the EPICS calls
                let ctx_clone = Arc::clone(&ctx);
                let tx_clone = tx.clone();
                
                // We do this in a blocking task to avoid blocking the async runtime
                tokio::task::spawn_blocking(move || {
                    for pv in pvs {
                        // We still lock for each PV, but now we are in a blocking thread
                        // so we don't stall the WebSocket handling
                        let response = do_get(&ctx_clone, &pv);
                        if tx_clone.send(response).is_err() {
                            break;
                        }
                    }
                }).await.ok();
            }
        }
    };

    // Task to handle incoming messages
    let read_task = {
        let ctx = Arc::clone(&ctx);
        let client_state = Arc::clone(&client_state);
        async move {
            while let Some(msg) = read.next().await {
                let msg = match msg {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("WebSocket error: {}", e);
                        break;
                    }
                };

                if msg.is_text() || msg.is_binary() {
                    let text = msg.to_text().unwrap_or("").to_string();
                    
                    let ctx = Arc::clone(&ctx);
                    let client_state = Arc::clone(&client_state);

                    // Spawn a task to handle the request so we don't block reading
                    tokio::spawn(async move {
                        let request: Request = match serde_json::from_str(&text) {
                            Ok(req) => req,
                            Err(e) => {
                                eprintln!("Failed to parse request: {}", e);
                                return;
                            }
                        };

                        // For blocking EPICS calls, use spawn_blocking
                        let client_state_clone = Arc::clone(&client_state);
                        let response = tokio::task::spawn_blocking(move || {
                            match request {
                                Request::Get { pv } => Some(do_get(&ctx, &pv)),
                                Request::Put { pv, value } => {
                                    let put_result = handle_put(&ctx, &pv, value);
                                    // After PUT, immediately get the readback value
                                    let readback = do_get(&ctx, &pv);
                                    
                                    // Send PUT result immediately
                                    let tx = client_state_clone.lock().unwrap().tx.clone();
                                    let _ = tx.send(put_result);
                                    
                                    Some(readback)
                                }
                                Request::Monitor { pv } => {
                                    client_state_clone.lock().unwrap().monitored_pvs.insert(pv.clone());
                                    println!("MONITOR started for {}", pv);
                                    // Return initial value
                                    Some(do_get(&ctx, &pv))
                                }
                                Request::Unmonitor { pv } => {
                                    client_state_clone.lock().unwrap().monitored_pvs.remove(&pv);
                                    println!("MONITOR stopped for {}", pv);
                                    None
                                }
                            }
                        }).await.unwrap_or(None);

                        if let Some(resp) = response {
                            let tx = client_state.lock().unwrap().tx.clone();
                            let _ = tx.send(resp);
                        }
                    });
                }
            }
        }
    };

    // Run all tasks concurrently, stop when read_task ends
    tokio::select! {
        _ = write_task => {},
        _ = poll_task => {},
        _ = read_task => {},
    }

    println!("WebSocket connection closed");
}

fn do_get(ctx: &Arc<Mutex<Context>>, pv: &str) -> Response {
    let mut ctx = ctx.lock().unwrap();
    
    match ctx.get(pv, 5.0) {
        Ok(value) => {
            let val = value.get_field_double("value").unwrap_or(0.0);
            let severity = value.get_field_int32("alarm.severity").ok();
            let status = value.get_field_int32("alarm.status").ok();
            let message = value.get_field_string("alarm.message").ok();
            
            Response::Value {
                pv: pv.to_string(),
                value: val,
                severity,
                status,
                message,
            }
        }
        Err(e) => {
            Response::Error {
                pv: pv.to_string(),
                message: format!("{}", e),
            }
        }
    }
}

fn handle_put(ctx: &Arc<Mutex<Context>>, pv: &str, value: f64) -> Response {
    let mut ctx = ctx.lock().unwrap();
    
    match ctx.put_double(pv, value, 5.0) {
        Ok(_) => {
            println!("PUT {}: {}", pv, value);
            Response::Success {
                pv: pv.to_string(),
                message: format!("Written: {}", value),
            }
        }
        Err(e) => {
            eprintln!("PUT {} failed: {}", pv, e);
            Response::Error {
                pv: pv.to_string(),
                message: format!("{}", e),
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("EPICS PVAccess WebSocket Bridge");
    println!("================================\n");

    // Create EPICS context
    let ctx = Context::from_env()?;
    let ctx = Arc::new(Mutex::new(ctx));
    println!("✓ EPICS context initialized");

    // Start WebSocket server
    let addr = "127.0.0.1:8765";
    let listener = TcpListener::bind(addr).await?;
    println!("✓ WebSocket server listening on ws://{}", addr);
    println!("\nWaiting for connections...\n");

    while let Ok((stream, addr)) = listener.accept().await {
        println!("Connection from: {}", addr);
        let ctx_clone = Arc::clone(&ctx);
        tokio::spawn(async move {
            handle_client(stream, ctx_clone).await;
        });
    }

    Ok(())
}
