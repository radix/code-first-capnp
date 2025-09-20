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

// Example showing backwards compatibility with extra fields
// This struct has removed some fields but maintains them in the schema
#[derive(Facet)]
#[facet(capnp:extra="oldUserId @1 :UInt64")]
#[facet(capnp:extra="deprecatedTimestamp @3 :UInt64")]
#[facet(capnp:extra="removedMetadata @6 :Text")]
struct UserProfileV2 {
    #[facet(capnp:id=0)]
    username: String,

    #[facet(capnp:id=2)]
    email: String,

    #[facet(capnp:id=4)]
    active: bool,

    #[facet(capnp:id=5)]
    preferences: Vec<String>,
}

#[derive(Facet)]
struct EmptyStruct;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Collect all the shapes we want to include in the schema
    let shapes = &[
        Company::SHAPE,
        Person::SHAPE,
        Status::SHAPE,
        EnumWithData::SHAPE,
        EmptyStruct::SHAPE,
        UserProfileV2::SHAPE,
    ];

    // Generate the complete schema file with a file ID
    // let file_id = 0xabcd1234u64;
    let file_id = 0xfbb45a811fbe71f5;
    let schema = code_first_capnp::build_capnp_file(file_id, shapes)?;
    println!("{}", schema);

    Ok(())
}
