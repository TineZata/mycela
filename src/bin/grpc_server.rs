use tonic::{transport::Server, Request, Response, Status};
use tonic_web::GrpcWebLayer;
use tower_http::cors::{CorsLayer, Any};
use std::collections::HashMap;
use tokio::sync::mpsc;
use std::sync::Arc;
use std::fs;

// Include generated protobuf code from src/generated
#[path = "../generated/epics.pv.rs"]
mod pv_service;

use pv_service::*;
use pv_service::pv_service_server::{PvService, PvServiceServer};

// Helper to load page configuration from JSON
fn load_page_config(path: &str) -> Result<PageConfig, Box<dyn std::error::Error>> {
    let json_str = fs::read_to_string(path)?;
    let config: PageConfig = serde_json::from_str(&json_str)?;
    Ok(config)
}

// Commands to send to the PVXS thread
enum PvCommand {
    GetPv {
        name: String,
        response_tx: tokio::sync::oneshot::Sender<Result<(f64, PvMetadata), String>>,
    },
    PutPv {
        name: String,
        value: f64,
        response_tx: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
}

// Service implementation using channel to communicate with PVXS thread
pub struct PvServiceImpl {
    command_tx: mpsc::UnboundedSender<PvCommand>,
}

impl PvServiceImpl {
    pub fn new(config_path: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let (command_tx, mut command_rx) = mpsc::unbounded_channel::<PvCommand>();
        
        // Load configuration to extract PV names
        let config_path = config_path.unwrap_or("examples/demo_config.json");
        let page_config = match load_page_config(config_path) {
            Ok(config) => {
                println!("Loaded page configuration from: {}", config_path);
                config
            }
            Err(e) => {
                eprintln!("Warning: Failed to load config from {}: {}", config_path, e);
                eprintln!("Using default configuration");
                // Return a default config with demo:motor:position
                PageConfig {
                    id: "default".to_string(),
                    title: "Default Config".to_string(),
                    description: String::new(),
                    widgets: vec![
                        WidgetConfig {
                            id: "motor_position_sp".to_string(),
                            pv_name: "demo:motor:position".to_string(),
                            r#type: 0,
                            config: None,
                            style: None,
                            label: "Motor Position".to_string(),
                            description: String::new(),
                            layout: None,
                        }
                    ],
                    layout: None,
                    style: None,
                }
            }
        };
        
        // Spawn PVXS thread
        std::thread::spawn(move || {
            use std::collections::HashMap;
            
            // Initialize PVXS server in this thread
            let mut server = match pvxs_sys::Server::from_env() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to create PVXS server: {}", e);
                    return;
                }
            };
            
            // Create PVXS client context for doing puts
            let mut client = match pvxs_sys::Context::from_env() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to create PVXS client context: {}", e);
                    return;
                }
            };
            println!("PVXS client context created for put operations");
            
            // Create PVs from configuration
            println!("\nCreating PVs from configuration:");
            for widget in &page_config.widgets {
                let pv_name = &widget.pv_name;
                
                // Default metadata - in a real system you'd extract from widget config
                let metadata = pvxs_sys::NTScalarMetadataBuilder::new()
                    .alarm(0, 0, "OK")
                    .display(pvxs_sys::DisplayMetadata {
                        limit_low: 0,
                        limit_high: 100,
                        description: widget.label.clone(),
                        units: "mm".to_string(),
                        precision: 2,
                    })
                    .value_alarm(pvxs_sys::ValueAlarmMetadata {
                        active: true,
                        low_alarm_limit: 5.0,
                        low_warning_limit: 10.0,
                        high_warning_limit: 90.0,
                        high_alarm_limit: 100.0,
                        low_alarm_severity: 2,
                        low_warning_severity: 1,
                        high_warning_severity: 1,
                        high_alarm_severity: 2,
                        hysteresis: 0,
                    });
                
                // Create PV with initial value of 50.0
                match server.create_pv_double(pv_name, 50.0, metadata) {
                    Ok(_) => println!("  ✓ Created PV: {}", pv_name),
                    Err(e) => eprintln!("  ✗ Failed to create PV {}: {}", pv_name, e),
                }
            }
            
            if let Err(e) = server.start() {
                eprintln!("Failed to start PVXS server: {}", e);
                return;
            }
            
            let port = server.tcp_port();
            println!("\nPVXS server started on TCP port {}", port);
            
            // Process commands - this is a blocking loop in the PVXS thread
            while let Some(cmd) = command_rx.blocking_recv() {
                match cmd {
                    PvCommand::GetPv { name, response_tx } => {
                        // Use PVXS client to get the actual PV value
                        println!("[PVXS Thread] GetPV: {} -> reading from PVXS", name);
                        match client.get(&name, 5.0) {
                            Ok(pv_value) => {
                                let value = pv_value.get_field_double("value").unwrap_or(0.0);
                                println!("[PVXS Thread] Got value from PVXS: {} = {}", name, value);
                                
                                let metadata = PvMetadata {
                                    alarm: Some(Alarm {
                                        severity: 0,
                                        status: 0,
                                        message: "OK".to_string(),
                                    }),
                                    display: Some(Display {
                                        limit_low: 0.0,
                                        limit_high: 100.0,
                                        description: "Demo Motor".to_string(),
                                        units: "mm".to_string(),
                                        precision: 2,
                                    }),
                                    control: Some(Control {
                                        limit_low: 0.0,
                                        limit_high: 100.0,
                                        min_step: 0.1,
                                    }),
                                    value_alarm: None,
                                };
                                let _ = response_tx.send(Ok((value, metadata)));
                            }
                            Err(e) => {
                                eprintln!("[PVXS Thread] Failed to get PV value: {}", e);
                                let _ = response_tx.send(Err(format!("PVXS error: {}", e)));
                            }
                        }
                    }
                    PvCommand::PutPv { name, value, response_tx } => {
                        // Use PVXS client to put the value
                        println!("[PVXS Thread] PutPV: {} = {} (writing via PVXS client)", name, value);
                        match client.put_double(&name, value, 5.0) {
                            Ok(_) => {
                                println!("[PVXS Thread] Successfully wrote value to PVXS PV");
                                let _ = response_tx.send(Ok(()));
                            }
                            Err(e) => {
                                eprintln!("[PVXS Thread] Failed to write PV value: {}", e);
                                let _ = response_tx.send(Err(format!("PVXS error: {}", e)));
                            }
                        }
                    }
                }
            }
        });
        
        Ok(Self { command_tx })
    }
}

#[tonic::async_trait]
impl PvService for PvServiceImpl {
    async fn get_pv(
        &self,
        request: Request<GetPvRequest>,
    ) -> Result<Response<PvValue>, Status> {
        let req = request.into_inner();
        println!("[gRPC Handler] GetPV request for: {} (timeout: {})", req.name, req.timeout);
        
        // Send command to PVXS thread
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        self.command_tx
            .send(PvCommand::GetPv {
                name: req.name.clone(),
                response_tx,
            })
            .map_err(|_| Status::internal("Failed to send command to PVXS thread"))?;
        
        // Wait for response
        let (value, metadata) = response_rx
            .await
            .map_err(|_| Status::internal("Failed to receive response from PVXS thread"))?
            .map_err(|e| Status::internal(format!("PVXS error: {}", e)))?;
        
        let pv_value = PvValue {
            name: req.name.clone(),
            value: Some(pv_value::Value::DoubleValue(value)),
            metadata: Some(metadata),
            timestamp_sec: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            timestamp_nsec: 0,
        };
        
        Ok(Response::new(pv_value))
    }

    async fn put_pv(
        &self,
        request: Request<PutPvRequest>,
    ) -> Result<Response<PutPvResponse>, Status> {
        let req = request.into_inner();
        println!("[gRPC Handler] PutPV request for: {} (timeout: {})", req.name, req.timeout);
        
        // Extract value
        let value = match req.value {
            Some(put_pv_request::Value::DoubleValue(v)) => {
                println!("[gRPC Handler] Extracted double value: {}", v);
                v
            },
            Some(put_pv_request::Value::Int32Value(v)) => {
                println!("[gRPC Handler] Extracted int32 value: {}", v);
                v as f64
            },
            _ => {
                println!("[gRPC Handler] ERROR: Unsupported value type");
                return Err(Status::invalid_argument("Unsupported value type"));
            }
        };
        
        // Send command to PVXS thread
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        self.command_tx
            .send(PvCommand::PutPv {
                name: req.name.clone(),
                value,
                response_tx,
            })
            .map_err(|_| Status::internal("Failed to send command to PVXS thread"))?;
        
        // Wait for response
        response_rx
            .await
            .map_err(|_| Status::internal("Failed to receive response from PVXS thread"))?
            .map_err(|e| Status::internal(format!("PVXS error: {}", e)))?;
        
        let response = PutPvResponse {
            success: true,
            error: String::new(),
        };
        
        Ok(Response::new(response))
    }

    async fn get_info(
        &self,
        request: Request<InfoRequest>,
    ) -> Result<Response<InfoResponse>, Status> {
        let req = request.into_inner();
        
        let response = InfoResponse {
            structure: format!("{{\"name\":\"{}\",\"type\":\"double\"}}", req.name),
            metadata: Some(PvMetadata::default()),
        };
        
        Ok(Response::new(response))
    }

    type MonitorPVStream = tokio_stream::wrappers::ReceiverStream<Result<MonitorEvent, Status>>;

    async fn monitor_pv(
        &self,
        request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::MonitorPVStream>, Status> {
        let req = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        // Spawn task to send periodic updates
        tokio::spawn(async move {
            for i in 0..100 {
                let event = MonitorEvent {
                    event: Some(monitor_event::Event::Value(PvValue {
                        name: req.name.clone(),
                        value: Some(pv_value::Value::DoubleValue(50.0 + (i as f64) * 0.1)),
                        metadata: None,
                        timestamp_sec: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64,
                        timestamp_nsec: 0,
                    })),
                };
                
                if tx.send(Ok(event)).await.is_err() {
                    break;
                }
                
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
        
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn batch_get(
        &self,
        request: Request<BatchGetRequest>,
    ) -> Result<Response<BatchGetResponse>, Status> {
        let req = request.into_inner();
        let mut values = HashMap::new();
        let mut errors = HashMap::new();
        
        for name in req.names {
            // Mock response
            values.insert(name.clone(), PvValue {
                name: name.clone(),
                value: Some(pv_value::Value::DoubleValue(42.0)),
                metadata: None,
                timestamp_sec: 0,
                timestamp_nsec: 0,
            });
        }
        
        Ok(Response::new(BatchGetResponse { values, errors }))
    }

    async fn get_page_config(
        &self,
        request: Request<PageConfigRequest>,
    ) -> Result<Response<PageConfig>, Status> {
        let req = request.into_inner();
        
        // Return a sample page configuration
        let config = PageConfig {
            id: req.page_id,
            title: "Demo Control Panel".to_string(),
            description: "Sample widget configuration".to_string(),
            widgets: vec![
                WidgetConfig {
                    id: "motor1".to_string(),
                    pv_name: "demo:motor:position".to_string(),
                    r#type: WidgetType::TextEntry as i32,
                    config: Some(widget_config::Config::TextEntry(TextEntryConfig {
                        show_units: true,
                        show_readback: true,
                        precision: 2,
                        min_value: 0.0,
                        max_value: 100.0,
                        placeholder: "Enter value".to_string(),
                        entry_style: None,
                        readback_style: None,
                        auto_submit: false,
                        debounce_ms: 300.0,
                    })),
                    style: None,
                    label: "Motor Position".to_string(),
                    description: "Motor position control".to_string(),
                    layout: None,
                },
            ],
            layout: None,
            style: None,
        };
        
        Ok(Response::new(config))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting gRPC-Web server for PV access...");
    
    // Allow config path from command line arg, or use default
    let config_path = std::env::args().nth(1);
    let config_path_str = config_path.as_deref();
    
    let addr = "0.0.0.0:50051".parse()?;
    let service = PvServiceImpl::new(config_path_str)?;
    
    println!("\ngRPC-Web server listening on {}", addr);
    println!("CORS enabled for all origins");
    println!("\nUsage: grpc-server [config.json]");
    println!("Default config: examples/demo_config.json\n");
    
    Server::builder()
        .accept_http1(true)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
                .expose_headers(Any)
                .max_age(std::time::Duration::from_secs(3600))
        )
        .layer(GrpcWebLayer::new())
        .add_service(PvServiceServer::new(service))
        .serve(addr)
        .await?;
    
    Ok(())
}
