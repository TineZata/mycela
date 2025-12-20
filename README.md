# EPICS Control System Web UI

**Rust + Axum + HTMX + pvxs-sys** - A modern, lightweight web interface for EPICS control systems.

## 🎯 Architecture

```
Browser (HTMX 14KB)
    ↓ HTML over HTTP  
Axum Server (Rust)
    ↓ PVXS Monitors
pvxs-sys (EPICS PVAccess)
    ↓ Real-time updates
EPICS IOCs
```

### Key Benefits

- ✅ **Simple & Fast** - Only 14KB HTMX, no complex JavaScript frameworks
- ✅ **Real-time Monitoring** - Persistent PVXS monitors with connection status tracking
- ✅ **Direct Integration** - FFI to pvxs-sys, no gRPC/protobuf overhead
- ✅ **Connection Aware** - Shows timeout/disconnected status for unavailable PVs
- ✅ **Single Binary** - Easy deployment, all assets self-hosted (airgapped ready)
- ✅ **~80% less code** - Compared to previous gRPC+WASM approach (~850 LOC vs ~3000 LOC)

## 🚀 Quick Start

### Prerequisites

- Rust 1.75+ (`rustup update`)
- pvxs-sys library (Rust FFI bindings to PVXS C++ library)
- EPICS IOC with PVs to monitor (optional for testing - will show disconnected status)

### Build and Run

```bash
# Build the server
cargo build --release

# Run the server
./target/release/server.exe

# Or for development with hot reload
cargo install cargo-watch
cargo watch -x run
```

Server starts at: **http://127.0.0.1:3000**

## 📁 Project Structure

```
ctrl-sys-widgets/
├── src/
│   ├── main.rs           # Axum server, routes, handlers
│   ├── widgets.rs        # Maud HTML templates for widgets
│   ├── pv_monitor.rs     # PVXS monitor manager with connection tracking
│   └── config.rs         # JSON configuration loader
├── static/
│   ├── htmx.min.js       # Self-hosted HTMX (14KB)
│   ├── htmx-sse.js       # Server-Sent Events extension
│   └── style.css         # Modern EPICS UI styling
├── examples/
│   └── demo_config.json  # Widget configuration
├── archive/              # Old gRPC+WASM implementation (archived)
└── Cargo.toml
```

## 🎨 Features

### Widget Types

- **Text Entry** - Editable numeric fields with readback
- **Slider** - Range-based control
- **Gauge** - Visual numeric display
- **LED** - Binary status indicator
- **Button** - Command execution (future)

### Real-time Monitoring

The server creates persistent PVXS monitors for each PV, providing:

- **Automatic reconnection** when PVs come online
- **Connection status tracking** (Connected/Disconnected/Timeout/Error)
- **Live value updates** pushed from EPICS
- **Visual feedback** with colored borders and status messages

### Connection Status UI

Widgets display connection state:
- 🟢 **Green border** - Connected and updating
- 🟠 **Orange border** - Timeout (PV not found)
- 🔴 **Red border** - Disconnected or error
- **Disabled inputs** when not connected
- **Status messages** showing specific errors

## 📝 Configuration

Widget configuration in `examples/demo_config.json`:

```json
{
  "id": "demo",
  "title": "Demo Control Screen",
  "widgets": [
    {
      "id": "motor_x_pos",
      "pv_name": "demo:motor:x",
      "type": "text_entry",
      "label": "Motor X Position"
    }
  ]
}
```

## 🔧 Development

### Hot Reload

```bash
cargo watch -x run
```

### Debug Logging

```bash
RUST_LOG=debug cargo run
```

### Testing Without IOCs

The UI gracefully handles missing PVs by showing timeout status. No IOC required to test the interface.

## 🏗️ Technical Details

### Stack

- **Axum 0.7** - Fast, ergonomic web framework
- **Maud 0.25** - Compile-time HTML templating (type-safe)
- **HTMX 1.9.10** - Declarative AJAX with HTML attributes
- **pvxs-sys** - Rust FFI to PVXS C++ library
- **Tokio** - Async runtime
- **DashMap** - Concurrent hashmap for PV value caching

### Monitor Architecture

Each PV gets a dedicated monitor thread that:

1. Creates PVXS monitor with connection/disconnection event handlers
2. Runs in `spawn_blocking` thread (blocking PVXS calls)
3. Pushes updates to shared `DashMap` cache
4. Handles `MonitorEvent::Connected`, `Disconnected`, `Timeout`, etc.
5. Updates connection status for UI feedback

### HTMX Polling

Widgets poll for updates every second using HTMX:

```html
<div class="widget" hx-get="/poll/widget/motor_x" hx-trigger="every 1s">
  <!-- Widget content auto-updates -->
</div>
```

Server returns fresh HTML fragments with current values and connection status.

## 📦 Deployment

### Build Release Binary

```bash
cargo build --release
```

Binary location: `target/release/server.exe` (~1.9MB)

### Required Files

```
deployment/
├── server.exe              # Binary
└── static/                 # Static assets
    ├── htmx.min.js
    ├── htmx-sse.js
    └── style.css
```

### Environment Variables

```bash
# EPICS environment (required for pvxs-sys)
EPICS_PVA_ADDR_LIST=192.168.1.100
EPICS_PVA_AUTO_ADDR_LIST=YES
```

## 🗂️ Archived Code

The previous gRPC-Web + WASM implementation is archived in `archive/old_grpc/` for reference. Key differences:

| Metric | Old (gRPC+WASM) | New (Axum+HTMX) |
|--------|----------------|-----------------|
| Lines of Code | ~3000 | ~850 (73% reduction) |
| Browser Assets | 2MB+ WASM | 14KB HTMX |
| Build Time | 2-3 min | 30 sec |
| Dependencies | 80+ crates | 20 crates |
| Complexity | High | Low |

## 📄 License

See [LICENSE](LICENSE) file.

## 🤝 Contributing

This is a demonstration project showing Rust+HTMX for EPICS control systems. For production use, consider adding:

- Authentication/authorization
- Alarm severity handling and colors
- Metadata extraction (units, ranges, precision)
- Archive integration for historical data
- Configuration UI for creating widgets
- Multi-screen navigation
- WebSocket/SSE for sub-second updates

## 🔗 Related Projects

- [pvxs-sys](https://github.com/your-org/pvxs-sys) - Rust FFI bindings to PVXS
- [EPICS](https://epics-controls.org/) - Experimental Physics and Industrial Control System
- [HTMX](https://htmx.org/) - High power tools for HTML
- [Axum](https://github.com/tokio-rs/axum) - Web framework for Rust
