use facet::Facet;

#[derive(Facet)]
struct Person {
    #[facet(capnp:id=0)]
    id: u64,
    
    #[facet(capnp:id=1,name=fullName)]
    name: String,
    
    #[facet(capnp:id=2)]
    email_addresses: Vec<String>,
    
    #[facet(capnp:id=3)]
    age: u16,
    
    // No capnp attribute - will auto-assign the next available ID (4)
    is_active: bool,
    
    #[facet(capnp:id=10)]
    score: f64,
    
    // This will get auto-assigned ID 5 (skipping the manually assigned 10)
    tags: Vec<String>,
}

#[derive(Facet)]
struct Company {
    #[facet(capnp:id=0,name=companyName)]
    name: String,
    
    #[facet(capnp:id=1)]
    employees: Vec<Person>,
    
    #[facet(capnp:id=2)]
    founded_year: u32,
    
    is_public: bool, // auto-assigned ID 3
}

#[derive(Facet)]
struct EmptyStruct;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Code-First Cap'n Proto Schema Generation ===\n");
    
    // Generate schema for Person
    println!("Person schema:");
    let person_schema = code_first_capnp::capnp_struct_for::<Person>()?;
    println!("{}", person_schema);
    
    // Generate schema for Company
    println!("Company schema:");
    let company_schema = code_first_capnp::capnp_struct_for::<Company>()?;
    println!("{}", company_schema);
    
    // Generate schema for empty struct
    println!("EmptyStruct schema:");
    let empty_schema = code_first_capnp::capnp_struct_for::<EmptyStruct>()?;
    println!("{}", empty_schema);

    
    Ok(())
}