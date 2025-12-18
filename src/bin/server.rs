use pvxs_sys::{Server, NTScalarMetadataBuilder, DisplayMetadata, ValueAlarmMetadata};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting EPICS PVAccess Server for Control System Widgets...");
    
    // Create server from environment (enables network discovery)
    let mut server = Server::from_env()?;
    
    // Create metadata with display and alarm limits
    let metadata = NTScalarMetadataBuilder::new()
        .alarm(0, 0, "OK")
        .display(DisplayMetadata {
            limit_low: 5,
            limit_high: 100,
            description: "Motor Position".to_string(),
            units: "mm".to_string(),
            precision: 2,
        })
        .value_alarm(ValueAlarmMetadata {
            active: true,
            low_alarm_limit: 5.0,        // LOLO
            low_warning_limit: 10.0,
            high_warning_limit: 90.0,
            high_alarm_limit: 100.0,     // HIHI
            low_alarm_severity: 2,
            low_warning_severity: 1,
            high_warning_severity: 1,
            high_alarm_severity: 2,
            hysteresis: 0,
        });
    
    // Create motor position PV with initial value of 50mm
    let _motor_pv = server.create_pv_double("wasm:test:motor:position", 50.0, metadata)?;
    
    // Start server
    server.start()?;
    let port = server.tcp_port();
    println!("Server started on TCP port {}", port);
    println!("Available PV:");
    println!("  - wasm:test:motor:position");
    println!("    Initial: 50 mm");
    println!("    LOLO: 5 mm");
    println!("    HIHI: 100 mm");
    println!("\nPress Ctrl+C to stop the server");
    
    // Keep server running - allow client to write values
    loop {
        sleep(Duration::from_secs(1)).await;
    }
}
