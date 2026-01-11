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

/// Scalar value type that can hold different data types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum NTType {
    Double(f64),
    Int32(i32),
    String(String),
    Enum { index: i16, choice: String },
}

impl NTType {
    /// Try to get as f64 for backwards compatibility
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            NTType::Double(v) => Some(*v),
            NTType::Int32(v) => Some(*v as f64),
            NTType::Enum { index, .. } => Some(*index as f64),
            NTType::String(_) => None,
        }
    }
    
    /// Format value as string for display
    pub fn to_display_string(&self, precision: Option<i32>) -> String {
        match self {
            NTType::Double(v) => {
                if let Some(prec) = precision {
                    format!("{:.prec$}", v, prec = prec as usize)
                } else {
                    format!("{:.6}", v)
                }
            }
            NTType::Int32(v) => v.to_string(),
            NTType::String(s) => s.clone(),
            NTType::Enum { choice, .. } => choice.clone(),
        }
    }
}

impl Default for NTType {
    fn default() -> Self {
        // TODO: Default should be string
        NTType::Double(0.0)
    }
}

/// Cached PV value with metadata and connection status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PvValue {
    pub name: String,
    pub value: NTType,
    pub timestamp: i64,
    pub connection_status: ConnectionStatus,
    pub alarm_severity: i32,
    pub alarm_status: i32,
    pub alarm_message: Option<String>,
    pub units: Option<String>,
    // Display metadata
    pub precision: Option<i32>,
    pub limit_low: Option<f64>,
    pub limit_high: Option<f64>,
    pub description: Option<String>,
    // Control metadata
    pub control_low: Option<f64>,
    pub control_high: Option<f64>,
    pub min_step: Option<f64>,
    // Alarm limits
    pub low_alarm_limit: Option<f64>,
    pub low_warning_limit: Option<f64>,
    pub high_alarm_limit: Option<f64>,
    pub high_warning_limit: Option<f64>,
    pub low_alarm_severity: Option<String>,
    pub low_warning_severity: Option<String>,
    pub high_alarm_severity: Option<String>,
    pub high_warning_severity: Option<String>,
    pub hysteresis: Option<i32>,
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
                value: NTType::default(),
                timestamp: 0,
                connection_status: ConnectionStatus::Disconnected,
                alarm_severity: 3, // Invalid
                alarm_status: 0,
                alarm_message: Some("Connecting...".to_string()),
                units: None,
                precision: None,
                limit_low: None,
                limit_high: None,
                description: None,
                control_low: None,
                control_high: None,
                min_step: None,
                low_alarm_limit: None,
                low_warning_limit: None,
                high_alarm_limit: None,
                high_warning_limit: None,
                low_alarm_severity: None,
                low_warning_severity: None,
                high_alarm_severity: None,
                high_warning_severity: None,
                hysteresis: None,
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
                value: NTType::default(),
                timestamp: 0,
                connection_status: ConnectionStatus::Error(format!("Monitor creation failed: {}", e)),
                alarm_severity: 3,
                alarm_status: 0,
                alarm_message: Some(format!("Error: {}", e)),
                units: None,
                precision: None,
                limit_low: None,
                limit_high: None,
                description: None,
                control_low: None,
                control_high: None,
                min_step: None,
                low_alarm_limit: None,
                low_warning_limit: None,
                high_alarm_limit: None,
                high_warning_limit: None,
                low_alarm_severity: None,
                low_warning_severity: None,
                high_alarm_severity: None,
                high_warning_severity: None,
                hysteresis: None,
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
                // Got data update - detect the type and extract appropriately
                // Check specific types first (double, int32, enum) before string, since string conversion is more permissive
                let nt_value = if let Ok(d) = value.get_field_double("value") {
                    // Double type
                    NTType::Double(d)
                } else if let Ok(i) = value.get_field_int32("value") {
                    // Int32 type
                    NTType::Int32(i)
                } else if let Ok(en) = value.get_field_enum("value") {
                    // Enum type
                    NTType::Enum {
                        index: en,
                        choice: value.get_field_string("value.choices").ok()
                            .and_then(|choices| {
                                let parts: Vec<&str> = choices.split(',').collect();
                                parts.get(en as usize).map(|s| s.to_string())
                            })
                            .unwrap_or_else(|| format!("Enum#{}", en)),
                }
                } else if let Ok(s) = value.get_field_string("value") {
                    // String type (check last since it's more permissive)
                    NTType::String(s)
                } else {
                    // Fallback to unknown string
                    NTType::String("??".to_string())
                };
                
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                
                // Extract alarm information from the value
                let alarm_severity = value.get_field_int32("alarm.severity").unwrap_or(0);
                let alarm_status = value.get_field_int32("alarm.status").unwrap_or(0);
                let alarm_message = value.get_field_string("alarm.message").ok();
                
                // Extract display metadata
                let units = value.get_field_string("display.units").ok();
                let precision = value.get_field_int32("display.precision").ok();
                let limit_low = value.get_field_double("display.limitLow").ok();
                let limit_high = value.get_field_double("display.limitHigh").ok();
                let description = value.get_field_string("display.description").ok();
                
                // Extract control metadata
                let control_low = value.get_field_double("control.limitLow").ok();
                let control_high = value.get_field_double("control.limitHigh").ok();
                let min_step = value.get_field_double("control.minStep").ok();
                
                // Extract alarm limits
                let low_alarm_limit = value.get_field_double("valueAlarm.lowAlarmLimit").ok();
                let low_warning_limit = value.get_field_double("valueAlarm.lowWarningLimit").ok();
                let high_alarm_limit = value.get_field_double("valueAlarm.highAlarmLimit").ok();
                let high_warning_limit = value.get_field_double("valueAlarm.highWarningLimit").ok();
                
                // Extract alarm severity strings
                let low_alarm_severity = value.get_field_string("valueAlarm.lowAlarmSeverity").ok();
                let low_warning_severity = value.get_field_string("valueAlarm.lowWarningSeverity").ok();
                let high_alarm_severity = value.get_field_string("valueAlarm.highAlarmSeverity").ok();
                let high_warning_severity = value.get_field_string("valueAlarm.highWarningSeverity").ok();
                let hysteresis = value.get_field_int32("valueAlarm.hysteresis").ok();
                
                values.insert(pv_name.clone(), PvValue {
                    name: pv_name.clone(),
                    value: nt_value.clone(),
                    timestamp,
                    connection_status: ConnectionStatus::Connected,
                    alarm_severity,
                    alarm_status,
                    alarm_message,
                    units,
                    precision,
                    limit_low,
                    limit_high,
                    description,
                    control_low,
                    control_high,
                    min_step,
                    low_alarm_limit,
                    low_warning_limit,
                    high_alarm_limit,
                    high_warning_limit,
                    low_alarm_severity,
                    low_warning_severity,
                    high_alarm_severity,
                    high_warning_severity,
                    hysteresis,
                });
                
                tracing::debug!("PV {} updated: value={}, alarm_severity={}", pv_name, nt_value.to_display_string(precision), alarm_severity);
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
