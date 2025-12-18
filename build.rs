use std::io::Result;

fn main() -> Result<()> {
    // Only compile protobuf when proto feature is enabled
    #[cfg(feature = "proto")]
    {
        // Check if protoc is available
        match std::env::var("PROTOC") {
            Ok(protoc_path) => {
                println!("cargo:warning=Using protoc from: {}", protoc_path);
            }
            Err(_) => {
                println!("cargo:warning=PROTOC not set, prost-build will search for it");
                println!("cargo:warning=If build fails, install protobuf-compiler:");
                println!("cargo:warning=  - Windows: scoop install protobuf or choco install protobuf");
                println!("cargo:warning=  - Linux: sudo apt install protobuf-compiler");
                println!("cargo:warning=  - macOS: brew install protobuf");
                println!("cargo:warning=Or download from: https://github.com/protocolbuffers/protobuf/releases");
            }
        }
        
        // Create output directory if it doesn't exist
        let out_dir = "src/generated";
        std::fs::create_dir_all(out_dir)?;
        
        // Compile protobuf files
        // For WASM builds, only generate prost code (no tonic)
        // For server builds, generate full tonic code
        let target = std::env::var("TARGET").unwrap_or_default();
        
        if target.contains("wasm32") {
            // WASM build - use prost_build (no tonic) for data types only
            // Compile both protos to get all message types, but no service code
            let temp_out = std::env::var("OUT_DIR").unwrap();
            match prost_build::Config::new()
                .compile_well_known_types()
                .extern_path(".google.protobuf", "::prost_types")
                .compile_protos(&["proto/widgets.proto", "proto/pv_service.proto"], &["proto"]) 
            {
                Ok(_) => {
                    println!("cargo:warning=Successfully compiled protobuf files for WASM");
                }
                Err(e) => {
                    println!("cargo:warning=Failed to compile protobuf: {}", e);
                    return Err(e);
                }
            }
            
            // Copy the generated file from OUT_DIR to src/generated
            let src_file = format!("{}/epics.pv.rs", temp_out);
            let dst_file = format!("{}/epics.pv.rs", out_dir);
            match std::fs::copy(&src_file, &dst_file) {
                Ok(_) => println!("cargo:warning=Copied prost-generated file from {} to {}", src_file, dst_file),
                Err(e) => {
                    println!("cargo:warning=Failed to copy file from {} to {}: {}", src_file, dst_file, e);
                    println!("cargo:warning=Listing OUT_DIR contents:");
                    if let Ok(entries) = std::fs::read_dir(&temp_out) {
                        for entry in entries.flatten() {
                            if let Some(name) = entry.file_name().to_str() {
                                if name.ends_with(".rs") {
                                    println!("cargo:warning=  Found: {}", name);
                                }
                            }
                        }
                    }
                    return Err(e);
                }
            }
        } else {
            // Native build - use tonic for gRPC support
            match tonic_build::configure()
                .build_server(true)
                .build_client(true)
                .out_dir(out_dir)
                .compile_protos(&["proto/widgets.proto", "proto/pv_service.proto"], &["proto"]) 
            {
                Ok(_) => {
                    println!("cargo:warning=Successfully compiled protobuf files with gRPC support");
                }
                Err(e) => {
                    println!("cargo:warning=Failed to compile protobuf: {}", e);
                    println!("cargo:warning=The proto feature requires protoc to be installed");
                    return Err(e);
                }
            }
        }
        
        println!("cargo:rerun-if-changed=proto/widgets.proto");
        println!("cargo:rerun-if-changed=proto/pv_service.proto");
    }
    
    #[cfg(not(feature = "proto"))]
    {
        println!("cargo:warning=Building without proto feature - widget factory disabled");
    }
    
    Ok(())
}
