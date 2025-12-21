use std::sync::{Arc, Mutex};
use crate::config::{WidgetConfig, ServerConfig};
use pvxs_sys::{Server, StaticSource, DisplayMetadata, ControlMetadata, ValueAlarmMetadata, NTScalarMetadataBuilder};

/// Wrapper to make Server Send (we ensure thread-safe access via Mutex)
struct ServerWrapper {
    server: Server,
    source: StaticSource,
}

unsafe impl Send for ServerWrapper {}

/// PV Server Manager - creates and manages PVXS server instances
pub struct PvServerManager {
    server: Arc<Mutex<Option<ServerWrapper>>>,
    widgets_config: Arc<Mutex<Vec<WidgetConfig>>>,
    is_running: Arc<Mutex<bool>>,
}

impl PvServerManager {
    /// Create a new PV server manager
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            server: Arc::new(Mutex::new(None)),
            widgets_config: Arc::new(Mutex::new(Vec::new())),
            is_running: Arc::new(Mutex::new(false)),
        })
    }
    
    /// Check if server is running
    pub fn is_running(&self) -> bool {
        *self.is_running.lock().unwrap()
    }
    
    /// Initialize and start the PVXS server with widgets from config
    pub fn start(&self, widgets: &[WidgetConfig]) -> Result<(), Box<dyn std::error::Error>> {
        // Check if already running
        {
            let is_running = self.is_running.lock().unwrap();
            if *is_running {
                return Err("Server is already running".into());
            }
        }
        
        tracing::info!("Initializing PVXS server...");
        
        // Store widgets config for later restart
        *self.widgets_config.lock().unwrap() = widgets.to_vec();
        
        // Create server from environment configuration
        let mut server = Server::from_env()?;
        tracing::info!("Server created from environment");
        
        // Create a static source
        let mut source = StaticSource::create()?;
        tracing::info!("Static source created");
        
        // Create PVs for each widget that has server configuration
        for widget in widgets {
            if let Some(server_config) = &widget.server {
                tracing::info!("Creating PV for: {}", widget.pv_name);
                
                // Create PV with metadata and add to source
                let mut pv = self.create_pv_with_metadata(&mut server, &widget.pv_name, &widget.data_type, server_config)?;
                source.add_pv(&widget.pv_name, &mut pv)?;
                
                tracing::info!("✓ Added PV: {}", widget.pv_name);
            }
        }
        
        // Add source to server
        server.add_source("static", &mut source, 0)?;
        tracing::info!("Added static source to server");
        
        // Start the server
        tracing::info!("Starting PVXS server...");
        server.start()?;
        
        // Store the server
        *self.server.lock().unwrap() = Some(ServerWrapper { server, source });
        *self.is_running.lock().unwrap() = true;
        
        tracing::info!("✅ PVXS server started successfully");
        tracing::info!("Server running on EPICS network");
        
        Ok(())
    }
    
    /// Create a PV with metadata
    fn create_pv_with_metadata(
        &self,
        server: &mut Server,
        pv_name: &str,
        data_type: &Option<String>,
        server_config: &ServerConfig,
    ) -> Result<pvxs_sys::SharedPV, Box<dyn std::error::Error>> {
        // Build metadata
        let mut metadata_builder = NTScalarMetadataBuilder::new();
        
        // Set alarm if specified
        if let Some(severity_str) = &server_config.alarm_serverity {
            let severity = parse_alarm_severity(severity_str);
            let status_str = server_config.alarm_status.as_ref()
                .map(|s| s.as_str())
                .unwrap_or("DEVICE");
            metadata_builder = metadata_builder.alarm(severity, 0, status_str);
        } else {
            metadata_builder = metadata_builder.alarm(0, 0, "OK");
        }
        
        // Add display metadata if available
        if let Some(metadata) = &server_config.metadata {
            if let Some(display) = &metadata.display {
                metadata_builder = metadata_builder.display(DisplayMetadata {
                    limit_low: display.limit_low as i64,
                    limit_high: display.limit_high as i64,
                    description: display.description.clone(),
                    units: display.units.clone(),
                    precision: display.precision,
                });
            }
            
            // Add control metadata if available
            if let Some(control) = &metadata.control {
                metadata_builder = metadata_builder.control(ControlMetadata {
                    limit_low: control.limit_low,
                    limit_high: control.limit_high,
                    min_step: 0.1,
                });
            }
            
            // Add alarm metadata if available
            if let Some(alarm) = &metadata.alarm {
                metadata_builder = metadata_builder.value_alarm(ValueAlarmMetadata {
                    active: true,
                    low_alarm_limit: alarm.low_alarm_limit,
                    low_warning_limit: alarm.low_warning_limit,
                    high_warning_limit: alarm.high_warning_limit,
                    high_alarm_limit: alarm.high_alarm_limit,
                    low_alarm_severity: parse_alarm_severity(&alarm.low_alarm_severity),
                    low_warning_severity: parse_alarm_severity(&alarm.low_warning_severity),
                    high_warning_severity: parse_alarm_severity(&alarm.high_warning_severity),
                    high_alarm_severity: parse_alarm_severity(&alarm.high_alarm_severity),
                    hysteresis: alarm.hysteresis as u8,
                });
            }
        }
        
        metadata_builder = metadata_builder.with_form(true);
        
        // Create PV with initial value (0.0) and metadata
        let pv = server.create_pv_double(pv_name, 0.0, metadata_builder)?;
        
        Ok(pv)
    }
    
    /// Stop the server
    pub fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut server_opt = self.server.lock().unwrap();
        
        if let Some(mut wrapper) = server_opt.take() {
            tracing::info!("Stopping PVXS server...");
            wrapper.server.stop()?;
            *self.is_running.lock().unwrap() = false;
            tracing::info!("✅ PVXS server stopped");
            Ok(())
        } else {
            Err("Server is not running".into())
        }
    }
}

/// Parse alarm severity string to code
fn parse_alarm_severity(severity: &str) -> i32 {
    match severity.to_uppercase().as_str() {
        "NONE" => 0,
        "MINOR" => 1,
        "MAJOR" => 2,
        "INVALID" => 3,
        _ => {
            tracing::warn!("Unknown alarm severity: {}, using NONE", severity);
            0
        }
    }
}

impl Drop for PvServerManager {
    fn drop(&mut self) {
        // Server runs for the lifetime of the process
    }
}
