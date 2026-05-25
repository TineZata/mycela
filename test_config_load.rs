// Quick test to verify config loading with improved error messages
// Run with: cargo run --bin test-config-load examples/test_missing_label.json

use mycela::config::ScreenConfig;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    let config_path = if args.len() > 1 {
        &args[1]
    } else {
        "examples/demo_config.json"
    };
    
    println!("Attempting to load config from: {}\n", config_path);
    
    match ScreenConfig::load(config_path) {
        Ok(config) => {
            println!(" SUCCESS! Loaded configuration:");
            println!("   Title: {}", config.title);
            println!("   Description: {}", config.description);
            println!("   Widgets: {}", config.widgets.len());
            for widget in &config.widgets {
                println!("      - {} ({:?}): {}", widget.id, widget.widget_type, widget.label);
            }
        }
        Err(e) => {
            eprintln!("❌ FAILED to load configuration:\n");
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
