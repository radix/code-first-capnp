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

    #[facet(capnp:id=7)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Facet)]
#[repr(u8)]
enum EnumWithData {
    MyText(#[facet(capnp:id=0)] String),
    Image {
        #[facet(capnp:id=1)]
        url: String,
        #[facet(capnp:id=2)]
        caption: String,
    },
    Video(#[facet(capnp:id=3)] String, #[facet(capnp:id=4)] u32),
}

#[derive(Facet)]
struct EmptyStruct;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("# Demo schema for code-first-capnp");
    println!();

    let company_schema = code_first_capnp::capnp_schema_for::<Company>()?;
    println!("{company_schema}");

    let person_schema = code_first_capnp::capnp_schema_for::<Person>()?;
    println!("{person_schema}");

    let status_schema = code_first_capnp::capnp_schema_for::<Status>()?;
    println!("{status_schema}");

    let message_schema = code_first_capnp::capnp_schema_for::<EnumWithData>()?;
    println!("{message_schema}");

    let empty_schema = code_first_capnp::capnp_schema_for::<EmptyStruct>()?;
    println!("{empty_schema}");

    Ok(())
}
