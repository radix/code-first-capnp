use code_first_capnp_macros::CapnpType;

#[derive(CapnpType)]
#[allow(dead_code)]
pub struct Person {
    #[capnp(id = 0)]
    id: u64,

    #[capnp(id = 1, name = "fullName")]
    name: String,

    #[capnp(id = 2)]
    email_addresses: Vec<String>,

    #[capnp(id = 3)]
    age: u16,

    #[capnp(id = 4)]
    is_active: bool,

    #[capnp(id = 7)]
    score: f64,

    #[capnp(id = 5)]
    tags: Vec<String>,

    #[capnp(id = 6)]
    status: Status,
}

#[derive(CapnpType)]
#[allow(dead_code)]
pub struct Company {
    #[capnp(id = 0, name = "companyName")]
    name: String,

    #[capnp(id = 1)]
    employees: Vec<Person>,

    #[capnp(id = 2)]
    founded_year: u32,

    #[capnp(id = 3)]
    is_public: bool,
}

#[allow(dead_code)]
#[derive(CapnpType)]
#[repr(u8)]
pub enum Status {
    #[capnp(id = 0)]
    Active,
    #[capnp(id = 1)]
    Inactive,
    #[capnp(id = 2)]
    Pending,
    #[capnp(id = 3)]
    Suspended,
}

#[allow(dead_code)]
#[derive(CapnpType)]
#[repr(u8)]
pub enum EnumWithData {
    MyText(#[capnp(id = 0)] String),
    Image {
        #[capnp(id = 1)]
        url: String,
        #[capnp(id = 2)]
        caption: String,
    },
    Video(#[capnp(id = 3)] String, #[capnp(id = 4)] u32),
}

// Example showing backwards compatibility with extra fields
// This struct has removed some fields but maintains them in the schema
#[derive(CapnpType)]
#[allow(dead_code)]
#[capnp(extra = "oldUserId @1 :UInt64")]
#[capnp(extra = "deprecatedTimestamp @3 :UInt64")]
#[capnp(extra = "removedMetadata @6 :Text")]
pub struct UserProfileV2 {
    #[capnp(id = 0)]
    username: String,

    #[capnp(id = 2)]
    email: String,

    #[capnp(id = 4)]
    active: bool,

    #[capnp(id = 5)]
    preferences: Vec<String>,
}

#[derive(CapnpType)]
pub struct EmptyStruct;

/// Generate the Cap'n Proto schema for all the demo types
pub fn generate_schema() -> Result<String, Box<dyn std::error::Error>> {
    // Collect all the schema items we want to include in the schema
    let items = &[
        Company::get_capnp_schema(),
        Person::get_capnp_schema(),
        Status::get_capnp_schema(),
        EnumWithData::get_capnp_schema(),
        EmptyStruct::get_capnp_schema(),
        UserProfileV2::get_capnp_schema(),
    ];

    // Generate the complete schema file with a file ID
    // this file ID was generated with `capnpc -i`
    let file_id = 0xfbb45a811fbe71f5;
    code_first_capnp::build_capnp_file(file_id, items)
}
