# mycela

*Named from mycelium — the vast, silent network that binds an ecosystem together.*

mycela grows where your devices live. Like the hyphal threads that connect every root in a forest, it weaves EPICS, Modbus, and future protocols into a single, coherent control fabric. No protocol is a second-class citizen. Built in Rust — because speed and memory safety are not optional in systems that matter.

## Architecture

```
Browser (HTMX, self-hosted)
    ↓ HTML over HTTP / SSE
Axum Server (Rust)
    ├─ pvxs-sys (EPICS PVAccess)  [feature: epics]
    └─ tokio-modbus (Modbus TCP)  [feature: modbus]
```

### Key Benefits

- **Simple & Fast** — No JavaScript frameworks; HTMX + SSE for real-time updates
- **Multi-protocol** — EPICS PVAccess and Modbus TCP supported out of the box
- **Alarm aware** — Full alarm severity display (MAJOR / MINOR / INVALID / OFFLINE)
- **Airgap ready** — All assets (HTMX, fonts, CSS) are self-hosted
- **Library-first** — Import `mycela` as a crate and build your own server

## Quick Start

### Prerequisites

- Rust 1.75+ (`rustup update`)
- For EPICS: `pvxs-sys` library built alongside this crate (`../pvxs-sys`)
- For Modbus: no extra system dependencies

### Build and Run the Demo Server

```bash
# Both protocols (default)
cargo run --example demo_server

# EPICS only
cargo run --example demo_server --no-default-features --features epics

# Modbus only
cargo run --example demo_server --no-default-features --features modbus
```

Server starts at: **http://127.0.0.1:3000**

## Project Structure

```
mycela/
├── src/
│   ├── lib.rs            # Crate root and feature gates
│   ├── channel.rs        # Protocol-independent ChannelValue type
│   ├── config.rs         # JSON config types (ScreenConfig, WidgetConfig, ProtocolConfig)
│   ├── epics_channel.rs  # PVXS monitor integration   [feature: epics]
│   ├── modbus_client.rs  # Modbus TCP connection pool  [feature: modbus]
│   ├── server_setup.rs   # Embedded PVXS server setup  [feature: epics]
│   └── widgets/          # Maud HTML widget renderers
│       ├── mod.rs
│       ├── button.rs
│       ├── chart.rs
│       ├── gauge.rs
│       ├── group.rs
│       ├── led.rs
│       ├── select.rs
│       ├── slider.rs
│       ├── text_entry.rs
│       ├── text_update.rs
│       └── toggle_button.rs
├── examples/
│   ├── demo_config.json       # Widget screen configuration
│   └── demo_server/
│       ├── main.rs            # Axum routes and handlers
│       ├── epics_simulator.rs # Simulated EPICS PV data
│       └── modbus_simulator.rs
├── static/
│   ├── htmx.min.js
│   ├── style.css
│   ├── tooltip.js
│   └── fonts/             # Self-hosted Inter + IBM Plex Mono
├── tests/                 # Unit test suite
└── Cargo.toml
```

## Widgets

| Widget | Description |
|--------|-------------|
| `text_entry` | Editable numeric/string field with write-back |
| `text_update` | Read-only live value display |
| `gauge` | SVG arc gauge with alarm bands |
| `led` | Binary status indicator |
| `slider` | Range control with configurable limits |
| `button` | Momentary command button |
| `toggle_button` | Latching on/off control |
| `select` | Enum drop-down |
| `chart` | Multi-series SVG line chart (up to 6 series) |
| `group` | Layout container for nested widgets |

## Connection Status

All widgets reflect channel state through border colour and status icons:

| State | Indicator |
|-------|-----------|
| Connected, no alarm | Green border |
| Minor alarm (Hi / Lo) | Orange border + warning icon |
| Major alarm (HiHi / LoLo) | Red border + alarm icon |
| Disconnected / offline | Cyan border, input disabled |
| Invalid / unknown | Grey icon |

## Configuration

Screen layout is defined in a JSON file (`examples/demo_config.json`):

```json
{
  "id": "demo",
  "title": "Demo Control Screen",
  "description": "...",
  "widgets": [
    {
      "id": "motor_x",
      "type": "text_entry",
      "label": "Motor X Position",
      "data_type": "double",
      "protocol": {
        "type": "epics-pva",
        "pv_name": "demo:double"
      },
      "metadata": {
        "display": { "limit_low": 0.0, "limit_high": 100.0, "units": "mm", "precision": 3 },
        "alarm": { "low_alarm_limit": 5.0, "high_alarm_limit": 95.0 }
      }
    },
    {
      "id": "pump_speed",
      "type": "slider",
      "label": "Pump Speed",
      "data_type": "double",
      "protocol": {
        "type": "modbus-tcp",
        "host": "192.168.1.10",
        "port": 502,
        "register": 1000,
        "register_type": "holding",
        "scale": 0.1
      }
    }
  ]
}
```

## Technical Stack

| Component | Version |
|-----------|---------|
| Axum | 0.8.8 |
| Maud (HTML templating) | 0.27.0 |
| Tokio | 1.51 |
| tokio-modbus | 0.17 |
| pvxs-sys | local path |
| DashMap | 6 |
| plotters (SVG) | 0.3 |

## Development

```bash
# Debug logging
$env:RUST_LOG="info"; cargo run --example demo_server

# Run tests
cargo test

# Build release
cargo build --release --example demo_server
```

### Environment Variables (EPICS)

```bash
EPICS_PVA_ADDR_LIST=192.168.1.100
EPICS_PVA_AUTO_ADDR_LIST=YES
```

## License

See [LICENSE](LICENSE) file.
