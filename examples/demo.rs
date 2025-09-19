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

    #[facet(capnp:id=4)]
    is_active: bool,

    #[facet(capnp:id=10)]
    score: f64,

    #[facet(capnp:id=5)]
    tags: Vec<String>,

    #[facet(capnp:id=6)]
    status: Status,
}

#[derive(Facet)]
struct Company {
    #[facet(capnp:id=0,name=companyName)]
    name: String,

    #[facet(capnp:id=1)]
    employees: Vec<Person>,

    #[facet(capnp:id=2)]
    founded_year: u32,

    #[facet(capnp:id=3)]
    is_public: bool,
}

#[derive(Facet)]
#[repr(u8)]
enum Status {
    #[facet(capnp:id=0)]
    Active,
    #[facet(capnp:id=1)]
    Inactive,
    #[facet(capnp:id=2)]
    Pending,
    #[facet(capnp:id=3)]
    Suspended,
}

#[derive(Facet)]
#[repr(u8)]
enum Message {
    #[facet(capnp:id=0)]
    Text(String),
    #[facet(capnp:id=1)]
    Image { url: String, caption: String },
    #[facet(capnp:id=2)]
    Video(String, u32), // url, duration_seconds
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

    // Generate union for Status enum
    println!("Status union:");
    let status_union = code_first_capnp::capnp_union_for::<Status>()?;
    println!("{}", status_union);

    // Generate union for Message enum with associated data
    println!("Message union:");
    let message_union = code_first_capnp::capnp_union_for::<Message>()?;
    println!("{}", message_union);

    // Generate helper structs for Message enum variants
    println!("Message variant structs:");
    let message_structs = code_first_capnp::capnp_enum_variant_structs_for::<Message>()?;
    println!("{}", message_structs);

    // Generate schema for empty struct
    println!("EmptyStruct schema:");
    let empty_schema = code_first_capnp::capnp_struct_for::<EmptyStruct>()?;
    println!("{}", empty_schema);

    Ok(())
}
