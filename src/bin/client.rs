use pvxs_sys::Context;
use std::error::Error;

fn read_pv(ctx: &mut Context, name: &str) -> Result<(), Box<dyn Error>> {
    println!("\nReading PV: {}", name);
    
    let value = ctx.get(name, 5.0)?;
    
    // Extract the main value
    let double_val = value.get_field_double("value")?;
    println!("  Value: {}", double_val);
    
    // Extract alarm metadata
    if let Ok(severity) = value.get_field_int32("alarm.severity") {
        println!("  Alarm Severity: {}", severity);
    }
    
    if let Ok(status) = value.get_field_int32("alarm.status") {
        println!("  Alarm Status: {}", status);
    }
    
    if let Ok(msg) = value.get_field_string("alarm.message") {
        println!("  Alarm Message: {}", msg);
    }
    
    Ok(())
}

fn write_pv(ctx: &mut Context, name: &str, value: f64) -> Result<(), Box<dyn Error>> {
    println!("\nWriting PV: {} = {}", name, value);
    
    ctx.put_double(name, value, 5.0)?;
    println!("  Write successful");
    
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("PVXS Client Test\n");
    
    // Create context from environment
    let mut ctx = Context::from_env()?;
    
    let pv_name = "wasm:test:motor:position";
    
    // Read initial value
    println!("=== Reading Initial Value ===");
    read_pv(&mut ctx, pv_name)?;
    
    // Write a new value (75mm - within range)
    println!("\n=== Writing Valid Value (75mm) ===");
    write_pv(&mut ctx, pv_name, 75.0)?;
    read_pv(&mut ctx, pv_name)?;
    
    // Write a value near HIHI limit
    println!("\n=== Writing Near HIHI (95mm) ===");
    write_pv(&mut ctx, pv_name, 95.0)?;
    read_pv(&mut ctx, pv_name)?;
    
    // Write a value near LOLO limit
    println!("\n=== Writing Near LOLO (7mm) ===");
    write_pv(&mut ctx, pv_name, 7.0)?;
    read_pv(&mut ctx, pv_name)?;
    
    println!("\nClient test complete!");
    Ok(())
}
