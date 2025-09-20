use std::env;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var("OUT_DIR")?;
    let out_dir_path = Path::new(&out_dir);
    
    // Rebuild if demo-types source changes
    println!("cargo:rerun-if-changed=../demo-types/src");
    println!("cargo:rerun-if-changed=../demo-types/Cargo.toml");
    
    // Generate the schema using demo-types library function
    let schema_content = demo_types::generate_schema()?;
    
    // Save the schema to a .capnp file
    let schema_path = out_dir_path.join("demo.capnp");
    fs::write(&schema_path, schema_content)?;
    
    // Use capnpc to generate Rust code from the schema
    capnpc::CompilerCommand::new()
        .src_prefix(&out_dir)
        .file(&schema_path)
        .run()?;
    
    Ok(())
}
