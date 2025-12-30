// Quick test to verify config loading
use serde_json;
use std::fs;

#[derive(serde::Deserialize, Debug)]
struct WidgetConfig {
    id: String,
    pv_name: String,
    #[serde(rename = "type")]
    widget_type: WidgetType,
    label: String,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
enum WidgetType {
    TextEntry,
    TextUpdate,
    Gauge,
    Led,
    Button,
    Slider,
    Chart,
}

#[derive(serde::Deserialize, Debug)]
struct ScreenConfig {
    id: String,
    title: String,
    description: String,
    widgets: Vec<WidgetConfig>,
}

fn main() {
    let content = fs::read_to_string("examples/demo_config.json")
        .expect("Failed to read config file");
    
    let config: ScreenConfig = serde_json::from_str(&content)
        .expect("Failed to parse JSON");
    
    println!("Loaded config: {}", config.title);
    println!("Number of widgets: {}", config.widgets.len());
    
    for (idx, widget) in config.widgets.iter().enumerate() {
        println!("  Widget {}: id={}, type={:?}, label='{}'", 
            idx, widget.id, widget.widget_type, widget.label);
    }
}
