use std::process::{Command, Child, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::io::{BufRead, BufReader};

struct ProcessManager {
    processes: Vec<(&'static str, Child)>,
}

impl ProcessManager {
    fn new() -> Self {
        Self { processes: Vec::new() }
    }

    fn spawn(&mut self, name: &'static str, mut cmd: Command) -> Result<(), String> {
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        match cmd.spawn() {
            Ok(child) => {
                println!("✓ Started {} (PID: {})", name, child.id());
                self.processes.push((name, child));
                Ok(())
            }
            Err(e) => Err(format!("Failed to start {}: {}", name, e))
        }
    }

    fn kill_all(&mut self) {
        println!("\n🛑 Shutting down all processes...");
        for (name, child) in &mut self.processes {
            match child.kill() {
                Ok(_) => println!("  ✓ Stopped {}", name),
                Err(e) => eprintln!("  ✗ Failed to stop {}: {}", name, e),
            }
        }
    }
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        self.kill_all();
    }
}

fn stream_output(name: &'static str, child: &mut Child) {
    // Stream stdout
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        let name = name.to_string();
        thread::spawn(move || {
            for line in reader.lines().map_while(Result::ok) {
                println!("[{}] {}", name, line);
            }
        });
    }

    // Stream stderr
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        let name = name.to_string();
        thread::spawn(move || {
            for line in reader.lines().map_while(Result::ok) {
                eprintln!("[{}] {}", name, line);
            }
        });
    }
}

fn find_python() -> Option<String> {
    // Try common Python commands
    for cmd in &["python", "python3", "py"] {
        if Command::new(cmd)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            return Some(cmd.to_string());
        }
    }
    None
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║   Control System Widgets - Unified Launcher            ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    // Set up Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    
    ctrlc::set_handler(move || {
        println!("\n\n⚠️  Ctrl+C received, initiating shutdown...");
        r.store(false, Ordering::SeqCst);
    })?;

    let mut manager = ProcessManager::new();

    // Get the path to the cargo executable
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    // 1. Start EPICS Server
    println!("📡 Starting EPICS Server...");
    let mut server_cmd = Command::new(&cargo);
    server_cmd
        .args(["run", "--bin", "pvxs-server", "--features", "server"])
        .current_dir(std::env::current_dir()?);
    manager.spawn("server", server_cmd)?;
    
    // Give server time to start
    thread::sleep(Duration::from_secs(2));

    // 2. Start WebSocket Bridge
    println!("🌉 Starting WebSocket Bridge...");
    let mut bridge_cmd = Command::new(&cargo);
    bridge_cmd
        .args(["run", "--bin", "pvxs-bridge", "--features", "bridge"])
        .current_dir(std::env::current_dir()?);
    manager.spawn("bridge", bridge_cmd)?;
    
    // Give bridge time to start
    thread::sleep(Duration::from_secs(2));

    // 3. Start HTTP Server for WASM
    println!("🌐 Starting HTTP Server for WASM UI...");
    let python = find_python().ok_or("Python not found in PATH")?;
    let mut http_cmd = Command::new(&python);
    http_cmd
        .args(["-m", "http.server", "8080"])
        .current_dir(std::env::current_dir()?);
    manager.spawn("http", http_cmd)?;

    // Stream output from all processes
    for (name, child) in &mut manager.processes {
        stream_output(name, child);
    }

    println!("\n════════════════════════════════════════════════════════");
    println!("✅ All services started!");
    println!("");
    println!("   🌐 Web UI:          http://localhost:8080/");
    println!("   🌉 WebSocket:       ws://127.0.0.1:8765");
    println!("   📡 EPICS Server:    localhost:5075");
    println!("");
    println!("   Press Ctrl+C to stop all services");
    println!("════════════════════════════════════════════════════════\n");

    // Wait for Ctrl+C or any process to exit
    while running.load(Ordering::SeqCst) {
        // Check if any process has exited
        for (name, child) in &mut manager.processes {
            match child.try_wait() {
                Ok(Some(status)) => {
                    eprintln!("\n⚠️  {} exited with status: {}", name, status);
                    running.store(false, Ordering::SeqCst);
                    break;
                }
                Ok(None) => {} // Still running
                Err(e) => {
                    eprintln!("\n⚠️  Error checking {} status: {}", name, e);
                }
            }
        }
        thread::sleep(Duration::from_millis(500));
    }

    // ProcessManager::drop will kill all processes
    Ok(())
}
