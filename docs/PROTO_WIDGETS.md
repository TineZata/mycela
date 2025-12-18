# Proto-Based Widget System

## Overview

This system allows you to define PVs, their metadata, and widget configurations in **protobuf** format, then dynamically instantiate them in your pages.

## Architecture

```
proto/pv_service.proto
    ↓ (prost-build compiles to Rust)
src/generated/pv_service.rs
    ↓ (used by)
src/widget_factory.rs ← Renders widgets from proto config
    ↓ (used by)
src/pages/proto_demo.rs ← Page that creates widget configs
```

## Usage

### 1. Define Widget Configuration in Code

```rust
use crate::generated::pv_service::*;

let text_entry_widget = WidgetConfig {
    id: "my_pv_input".to_string(),
    pv_name: "motor:position:sp".to_string(),
    r#type: WidgetType::TextEntry as i32,
    config: Some(widget_config::Config::TextEntry(TextEntryConfig {
        show_units: true,
        show_readback: true,
        precision: 2,
        min_value: -100.0,
        max_value: 100.0,
        placeholder: "Enter position...".to_string(),
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
    label: "Motor Position".to_string(),
    description: "Setpoint for motor position in mm".to_string(),
    layout: None,
};
```

### 2. Render Individual Widget

```rust
use crate::widget_factory;

let html = widget_factory::render_widget(&text_entry_widget)?;
```

### 3. Render Entire Page from Config

```rust
let page_config = PageConfig {
    id: "my_page".to_string(),
    title: "Control Dashboard".to_string(),
    description: "Dynamically generated page".to_string(),
    widgets: vec![
        text_entry_widget,
        gauge_widget,
        chart_widget,
        // ... more widgets
    ],
    layout: Some(PageLayout {
        r#type: LayoutType::Grid as i32,
        columns: 2,
        gap: 20,
    }),
    style: Some(PageStyle {
        background: "linear-gradient(135deg, #1e1e1e 0%, #2d2d2d 100%)".to_string(),
        text_color: "#ffffff".to_string(),
        font_family: "system-ui".to_string(),
    }),
};

let page_html = widget_factory::render_page(&page_config);
```

### 4. Use in a Page Module

```rust
// src/pages/my_page.rs
use crate::generated::pv_service::*;
use crate::widget_factory;

pub fn render() -> String {
    let page_config = create_my_page_config();
    widget_factory::render_page(&page_config)
}

fn create_my_page_config() -> PageConfig {
    PageConfig {
        widgets: vec![
            // Define your widgets here
        ],
        // ... rest of config
    }
}
```

## Widget Types Supported

### TextEntry
- Input field with validation
- Units display
- Live readback value
- Alarm visualization
- Custom styling

### Gauge
- Min/max ranges
- Color zones
- Tick marks
- Value display

### Chart
- Time-series plotting
- Configurable history
- Auto-scaling
- Custom line styles

### Button
- PUT value on click
- Confirmation dialogs
- Custom styling

### LED
- State-based coloring
- Labels
- Multiple states

### Slider
- Horizontal/vertical
- Step control
- Value display
- Min/max ranges

## Configuration Loading

Future enhancement: Load configs from gRPC server or JSON files:

```rust
// From gRPC
async fn load_page_from_server(page_id: &str) -> Result<PageConfig, Error> {
    let client = PVServiceClient::new("http://localhost:8080");
    let request = PageConfigRequest {
        page_id: page_id.to_string(),
    };
    let response = client.get_page_config(request).await?;
    Ok(response.into_inner())
}

// From JSON (with serde)
fn load_page_from_json(json: &str) -> Result<PageConfig, Error> {
    serde_json::from_str(json)
}
```

## Benefits

1. **Type Safety**: Compile-time checking of widget configurations
2. **Declarative**: Define UI structure in data, not code
3. **Reusable**: Share widget configs across pages
4. **Dynamic**: Load configurations at runtime from server
5. **Versioned**: Protobuf handles schema evolution
6. **Cross-Language**: Same proto definitions for Rust frontend & backend

## Building

```bash
# Build with protobuf support (default)
cargo build --target wasm32-unknown-unknown --release

# The build.rs script will:
# 1. Compile proto/pv_service.proto
# 2. Generate src/generated/pv_service.rs
# 3. Make types available to widget_factory
```

## Example: Creating a Page Programmatically

See `src/pages/proto_demo.rs` for a complete example with multiple widget types.

```rust
// In your page file
pub fn render() -> String {
    let widgets = vec![
        create_text_entry_widget("pv1", "Position"),
        create_gauge_widget("pv2", "Pressure"),
        create_led_widget("pv3", "Status"),
    ];
    
    let page = PageConfig {
        title: "My Dashboard".to_string(),
        widgets,
        layout: Some(PageLayout {
            r#type: LayoutType::Grid as i32,
            columns: 3,
            gap: 20,
        }),
        ..Default::default()
    };
    
    widget_factory::render_page(&page)
}
```

## Next Steps

- [ ] Add JSON/protobuf file loading support
- [ ] Implement gRPC GetPageConfig endpoint
- [ ] Add widget validation rules
- [ ] Create visual config editor
- [ ] Add more widget types (table, dropdown, etc.)
