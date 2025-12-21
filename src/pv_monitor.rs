use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashSet;
use std::sync::Mutex;

/// PV connection status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Timeout,
    Error(String),
}

/// Cached PV value with metadata and connection status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PvValue {
    pub name: String,
    pub value: f64,
    pub timestamp: i64,
    pub connection_status: ConnectionStatus,
    pub alarm_severity: i32,
    pub alarm_status: i32,
    pub alarm_message: Option<String>,
    pub units: Option<String>,
}

/// Monitor manager - creates and maintains persistent PVXS monitors
pub struct PvMonitorManager {
    values: Arc<DashMap<String, PvValue>>,
    active_monitors: Arc<Mutex<HashSet<String>>>,
    client: Arc<RwLock<pvxs_sys::Context>>,
}

impl PvMonitorManager {
    pub fn new(client: Arc<RwLock<pvxs_sys::Context>>) -> Self {
        Self {
            values: Arc::new(DashMap::new()),
            active_monitors: Arc::new(Mutex::new(HashSet::new())),
            client,
        }
    }
    
    /// Get current value for a PV (creates monitor if doesn't exist)
    pub async fn get_value(&self, pv_name: &str) -> PvValue {
        // Check if we already have a value
        if let Some(entry) = self.values.get(pv_name) {
            return entry.value().clone();
        }
        
        // Check if monitor is already running for this PV
        let should_start = {
            let mut monitors = self.active_monitors.lock().unwrap();
            if monitors.contains(pv_name) {
                false // Monitor already exists
            } else {
                monitors.insert(pv_name.to_string());
                true // Start new monitor
            }
        };
        
        if should_start {
            // Start monitoring this PV in background
            self.start_monitor(pv_name).await;
        }
        
        // Return disconnected state initially (or cached value if it exists now)
        if let Some(entry) = self.values.get(pv_name) {
            entry.value().clone()
        } else {
            PvValue {
                name: pv_name.to_string(),
                value: 0.0,
                timestamp: 0,
                connection_status: ConnectionStatus::Disconnected,
                alarm_severity: 3, // Invalid
                alarm_status: 0,
                alarm_message: Some("Connecting...".to_string()),
                units: None,
            }
        }
    }
    
    /// Start a background monitor for a PV
    async fn start_monitor(&self, pv_name: &str) {
        let pv_name = pv_name.to_string();
        let values = self.values.clone();
        let client = self.client.clone();
        
        tokio::spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                monitor_pv_loop(pv_name, values, client)
            }).await;
            
            if let Err(e) = result {
                tracing::error!("Monitor task failed: {}", e);
            }
        });
    }
}

/// Monitor loop - runs in blocking thread
fn monitor_pv_loop(
    pv_name: String,
    values: Arc<DashMap<String, PvValue>>,
    client_arc: Arc<RwLock<pvxs_sys::Context>>,
) {
    tracing::info!("Starting monitor for PV: {}", pv_name);
    
    // Create monitor
    let mut client = client_arc.blocking_write();
    let monitor_result = client.monitor_builder(&pv_name)
        .and_then(|builder| {
            builder
                .connect_exception(true)  // Get connection events
                .disconnect_exception(true)  // Get disconnection events
                .exec()
        });
    
    let mut monitor = match monitor_result {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("Failed to create monitor for {}: {}", pv_name, e);
            values.insert(pv_name.clone(), PvValue {
                name: pv_name,
                value: 0.0,
                timestamp: 0,
                connection_status: ConnectionStatus::Error(format!("Monitor creation failed: {}", e)),
                alarm_severity: 3,
                alarm_status: 0,
                alarm_message: Some(format!("Error: {}", e)),
                units: None,
            });
            return;
        }
    };
    
    // Start the monitor
    if let Err(e) = monitor.start() {
        tracing::error!("Failed to start monitor for {}: {}", pv_name, e);
        return;
    }
    
    // Release the write lock
    drop(client);
    
    tracing::info!("Monitor started successfully for: {}", pv_name);
    
    // Main monitoring loop
    loop {
        match monitor.pop() {
            Ok(Some(value)) => {
                // Got data update
                let double_value = value.get_field_double("value").unwrap_or(0.0);
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                
                // Extract alarm information from the value
                let alarm_severity = value.get_field_int32("alarm.severity").unwrap_or(0);
                let alarm_status = value.get_field_int32("alarm.status").unwrap_or(0);
                let alarm_message = value.get_field_string("alarm.message").ok();
                let units = value.get_field_string("display.units").ok();
                
                values.insert(pv_name.clone(), PvValue {
                    name: pv_name.clone(),
                    value: double_value,
                    timestamp,
                    connection_status: ConnectionStatus::Connected,
                    alarm_severity,
                    alarm_status,
                    alarm_message,
                    units,
                });
                
                tracing::debug!("PV {} updated: value={}, alarm_severity={}", pv_name, double_value, alarm_severity);
            }
            Ok(None) => {
                // No data available, keep polling
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(pvxs_sys::MonitorEvent::Connected(msg)) => {
                tracing::info!("PV {} connected: {}", pv_name, msg);
                values.entry(pv_name.clone()).and_modify(|v| {
                    v.connection_status = ConnectionStatus::Connected;
                });
            }
            Err(pvxs_sys::MonitorEvent::Disconnected(msg)) => {
                tracing::warn!("PV {} disconnected: {}", pv_name, msg);
                values.entry(pv_name.clone()).and_modify(|v| {
                    v.connection_status = ConnectionStatus::Disconnected;
                    v.alarm_message = Some("PV Disconnected".to_string());
                });
            }
            Err(pvxs_sys::MonitorEvent::Finished(msg)) => {
                tracing::info!("PV {} monitor finished: {}", pv_name, msg);
                break;
            }
            Err(pvxs_sys::MonitorEvent::RemoteError(msg)) => {
                tracing::error!("PV {} remote error: {}", pv_name, msg);
                values.entry(pv_name.clone()).and_modify(|v| {
                    v.connection_status = ConnectionStatus::Error(msg.to_string());
                });
            }
            Err(pvxs_sys::MonitorEvent::ClientError(msg)) => {
                tracing::error!("PV {} client error: {}", pv_name, msg);
                values.entry(pv_name.clone()).and_modify(|v| {
                    v.connection_status = ConnectionStatus::Error(msg.to_string());
                });
                // For timeout errors, mark as timeout
                if msg.to_lowercase().contains("timeout") {
                    values.entry(pv_name.clone()).and_modify(|v| {
                        v.connection_status = ConnectionStatus::Timeout;
                        v.alarm_message = Some("PV Not Found (Timeout)".to_string());
                    });
                }
            }
        }
    }
    
    tracing::info!("Monitor loop ended for: {}", pv_name);
}
