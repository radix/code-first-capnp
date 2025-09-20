use demo_types::generate_schema;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = generate_schema()?;
    println!("{}", schema);
    Ok(())
}
