# Proto-Based Widget System - Quick Start

## What This Gives You

Define your **PV + metadata + widget configuration** in a single protobuf schema, then dynamically instantiate widgets in any page.

```rust
// Define widget configuration
let widget = WidgetConfig {
    id: "motor_pos",
    pv_name: "motor:position:sp",
    type: WidgetType::TextEntry,
    config: TextEntryConfig {
        show_units: true,
        show_readback: true,
        precision: 2,
        // ... more config
    },
    label: "Motor Position",
    // ... styling, layout
};

// Render it
let html = widget_factory::render_widget(&widget)?;
```

## Build Instructions

### 1. Install Dependencies

```bash
# Rust toolchain with WASM target
rustup target add wasm32-unknown-unknown

# wasm-bindgen CLI (if not installed)
cargo install wasm-bindgen-cli
```

### 2. Build the Project

```bash
# Build with proto feature enabled (default)
cargo build --target wasm32-unknown-unknown --release

# Generate JS bindings
wasm-bindgen target/wasm32-unknown-unknown/release/ctrl_sys_widgets.wasm \
  --out-dir pkg \
  --target web
```

### 3. Run the Demo

```bash
# Serve with any static file server
python -m http.server 8000

# Or use a Rust server
cargo install miniserve
miniserve . --index proto_demo.html
```

Then open: `http://localhost:8000/proto_demo.html`

## Project Structure

```
ctrl-sys-widgets/
├── proto/
│   └── pv_service.proto          # Protobuf schema (PV + widgets)
├── src/
│   ├── lib.rs                    # Main WASM entry
│   ├── widget_factory.rs         # Dynamic widget renderer
│   ├── generated/                # Generated protobuf code (auto)
│   │   └── pv_service.rs
│   ├── inputs/
│   │   ├── mod.rs
│   │   └── text_entry.rs         # Text entry widget implementation
│   └── pages/
│       ├── mod.rs
│       ├── home.rs               # Original WebSocket demo
│       └── proto_demo.rs         # Proto-based widget demo
├── build.rs                      # Protobuf compilation script
├── Cargo.toml                    # Dependencies
├── proto_demo.html               # Demo page
└── examples/
    └── beamline_config.json      # Example widget config
```

## Usage Examples

### Example 1: Create a Single Widget

```rust
use crate::generated::pv_service::*;
use crate::widget_factory;

// Define configuration
let config = WidgetConfig {
    id: "my_widget".to_string(),
    pv_name: "MY:PV:NAME".to_string(),
    r#type: WidgetType::TextEntry as i32,
    config: Some(widget_config::Config::TextEntry(TextEntryConfig {
        show_units: true,
        show_readback: true,
        precision: 3,
        min_value: 0.0,
        max_value: 100.0,
        placeholder: "Enter value...".to_string(),
        entry_style: Some(TextEntryStyle {
            background_color: "#e6f3ff".to_string(),
            text_color: "#333".to_string(),
            border_style: "solid".to_string(),
            font_size: 15,
            font_family: "monospace".to_string(),
        }),
        readback_style: Some(ReadbackStyle {
            background_color: "#f0f0f0".to_string(),
            text_color: "#00cc66".to_string(),
            show_alarm_border: true,
            alarm_viz: AlarmVisualization::IconAndBorder as i32,
        }),
        auto_submit: false,
        debounce_ms: 0.0,
    })),
    style: None,
    label: "My PV Control".to_string(),
    description: "Description here".to_string(),
    layout: None,
};

// Render to HTML
let html = widget_factory::render_widget(&config).unwrap();
```

### Example 2: Create a Page with Multiple Widgets

```rust
use crate::generated::pv_service::*;
use crate::widget_factory;

pub fn render() -> String {
    let page_config = PageConfig {
        id: "dashboard".to_string(),
        title: "My Control Dashboard".to_string(),
        description: "Dynamically generated from protobuf".to_string(),
        
        widgets: vec![
            // Text entry widget
            WidgetConfig {
                id: "widget1".to_string(),
                pv_name: "pv1".to_string(),
                r#type: WidgetType::TextEntry as i32,
                config: Some(widget_config::Config::TextEntry(/* config */)),
                label: "Widget 1".to_string(),
                // ...
            },
            
            // Gauge widget
            WidgetConfig {
                id: "widget2".to_string(),
                pv_name: "pv2".to_string(),
                r#type: WidgetType::Gauge as i32,
                config: Some(widget_config::Config::Gauge(GaugeConfig {
                    min_value: 0.0,
                    max_value: 100.0,
                    num_ticks: 10,
                    show_needle: true,
                    show_value_text: true,
                    ranges: vec![],
                })),
                label: "Pressure Gauge".to_string(),
                // ...
            },
            
            // Add more widgets...
        ],
        
        layout: Some(PageLayout {
            r#type: LayoutType::Grid as i32,
            columns: 2,
            gap: 20,
        }),
        
        style: Some(PageStyle {
            background: "linear-gradient(135deg, #1e1e1e, #2d2d2d)".to_string(),
            text_color: "#ffffff".to_string(),
            font_family: "system-ui".to_string(),
        }),
    };
    
    widget_factory::render_page(&page_config)
}
```

### Example 3: Use in Your Page Module

```rust
// src/pages/my_page.rs
use crate::generated::pv_service::*;
use crate::widget_factory;

pub fn render() -> String {
    let widgets = vec![
        create_motor_control("motor1", "Motor 1 Position"),
        create_motor_control("motor2", "Motor 2 Position"),
        create_pressure_gauge("vacuum1", "Chamber Pressure"),
        create_status_led("interlock1", "Interlock Status"),
    ];
    
    let page = PageConfig {
        title: "Motor Control".to_string(),
        description: "Control system for beamline motors".to_string(),
        widgets,
        layout: Some(PageLayout {
            r#type: LayoutType::Grid as i32,
            columns: 2,
            gap: 20,
        }),
        style: Some(PageStyle {
            background: "linear-gradient(135deg, #1e1e1e, #2d2d2d)".to_string(),
            text_color: "#fff".to_string(),
            font_family: "system-ui".to_string(),
        }),
    };
    
    widget_factory::render_page(&page)
}

fn create_motor_control(pv: &str, label: &str) -> WidgetConfig {
    WidgetConfig {
        id: format!("{}_widget", pv),
        pv_name: format!("SR:C01-{}:Pos-SP", pv),
        r#type: WidgetType::TextEntry as i32,
        config: Some(widget_config::Config::TextEntry(TextEntryConfig {
            show_units: true,
            show_readback: true,
            precision: 2,
            // ... rest of config
        })),
        label: label.to_string(),
        // ...
    }
}
```

## Widget Types Available

### 1. TextEntry
- Input field with validation
- Units display
- Live readback value
- Alarm visualization (icon + border)
- Custom colors and fonts

### 2. Gauge
- Radial or linear gauge
- Configurable ranges with colors
- Tick marks
- Min/max values

### 3. Chart (Placeholder)
- Time-series data plotting
- Configurable history
- Auto-scaling

### 4. Button
- PUT value on click
- Optional confirmation
- Custom styling

### 5. LED
- State-based indicator
- Multiple colors
- Label display

### 6. Slider
- Horizontal/vertical
- Step control
- Value display

## Protobuf Schema

The complete schema is in [proto/pv_service.proto](proto/pv_service.proto). Key messages:

- `WidgetConfig` - Base configuration for all widgets
- `TextEntryConfig` - Text entry specific options
- `GaugeConfig`, `ChartConfig`, etc. - Other widget types
- `PageConfig` - Complete page configuration
- `PVValue`, `PVMetadata` - EPICS PV data structures

## Future Enhancements

- [ ] Load configs from gRPC server via `GetPageConfig` RPC
- [ ] Parse JSON configs and convert to protobuf
- [ ] Add more widget types (table, dropdown, image)
- [ ] Real PV connection via gRPC-Web
- [ ] Widget validation and error handling
- [ ] Visual config editor

## Integration with gRPC-Web

The protobuf schema includes both widget configuration AND the PVService RPC definitions. This means:

1. **Frontend (WASM)** uses widget configs to render UI
2. **Backend (gRPC server)** can serve widget configs via `GetPageConfig` RPC
3. **Same schema** for both UI definition and PV communication

```rust
// Future: Load page config from gRPC server
let client = PVServiceClient::new("http://localhost:8080");
let request = PageConfigRequest {
    page_id: "motor_control".to_string(),
};
let page_config = client.get_page_config(request).await?;
let html = widget_factory::render_page(&page_config);
```

## Documentation

- [Proto Widget Guide](docs/PROTO_WIDGETS.md) - Detailed documentation
- [gRPC-Web Integration](GRPC_WEB_GUIDE.md) - Server architecture
- [Example Config](examples/beamline_config.json) - JSON example

## Questions?

The proto-based system gives you:
✅ Type-safe widget definitions
✅ Dynamic page generation
✅ Declarative UI configuration
✅ Cross-language compatibility (same proto for Rust + TypeScript)
✅ Version control friendly (text-based configs)
✅ Server-side page management

Try modifying `src/pages/proto_demo.rs` to see how easy it is to create new widgets!
