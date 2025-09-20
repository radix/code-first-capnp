# Code-First Cap'n Proto for Rust

Generate Cap'n Proto schemas directly from Rust types using proc macros, with support for defining types and using generated capnp code in the same crate.

## Overview

This library allows you to define your data structures in Rust and automatically generate corresponding Cap'n Proto schemas at compile time. No need to maintain separate `.capnp` files - your Rust types are the source of truth. The new single-crate approach eliminates the need for build scripts and separate type/usage crates.

## Features

- **Single-crate workflow** - define types and use generated capnp code in the same crate (no build scripts needed!)
- **Compile-time generation** using proc macros (zero runtime overhead)
- **Automatic schema compilation** with integrated capnpc invocation
- **Deterministic output** with proper ordering of generated schema items
- **Automatic field naming** with snake_case to camelCase conversion
- **Manual field IDs** with `#[capnp(id=N)]` attributes
- **Custom field names** with `#[capnp(name="customName")]`
- **Enum support** unit variants become void types, data variants become union groups
- **Backwards compatibility** with `#[capnp(extra="field @id :Type")]` for deprecated fields
- **Schema validation** with duplicate ID detection
- **Type safety** with full Rust type system integration

## Quick Start - Single Crate Approach (Recommended)

Add to your `Cargo.toml`:

```toml
[dependencies]
code-first-capnp = "0.1.0"
capnp = "0.21"
```

Define your types and use the generated capnp code all in one crate:

```rust
use code_first_capnp_macros::CapnpType;
use code_first_capnp::{capnp_schema_file, complete_capnp_schema};

// Initialize the schema file with a unique file ID
capnp_schema_file!("demo.capnp", 0xfbb45a811fbe71f5);

#[derive(CapnpType)]
#[capnp(file = "demo.capnp")]  // This type will be added to demo.capnp
pub struct Person {
    #[capnp(id = 0)]
    pub id: u64,

    #[capnp(id = 1, name = "fullName")]
    pub name: String,

    #[capnp(id = 2)]
    pub email_addresses: Vec<String>,

    #[capnp(id = 3)]
    pub age: u16,

    #[capnp(id = 4)]
    pub is_active: bool,

    #[capnp(id = 5)]
    pub status: Status,
}

#[derive(CapnpType)]
#[capnp(file = "demo.capnp")]
pub enum Status {
    #[capnp(id = 0)]
    Active,
    #[capnp(id = 1)]
    Inactive,
    #[capnp(id = 2)]
    Pending,
}

// Complete the schema and generate the capnp module
complete_capnp_schema!("demo.capnp", pub mod demo_capnp);

// Now you can use the generated types!
fn main() -> capnp::Result<()> {
    let mut message = capnp::message::Builder::new_default();
    let mut person = message.init_root::<demo_capnp::person::Builder>();
    
    person.set_id(12345);
    person.set_full_name("John Doe");
    person.set_age(30);
    person.set_is_active(true);
    
    println!("Person ID: {}", person.reborrow().get_id());
    Ok(())
}
```

## How It Works

1. **`capnp_schema_file!("demo.capnp", file_id)`** - Creates the schema file and initializes it with the file ID
2. **`#[capnp(file = "demo.capnp")]`** - Each `CapnpType` derive with this attribute adds its schema to the specified file
3. **`complete_capnp_schema!("demo.capnp", pub mod demo_capnp)`** - Compiles the schema with capnpc and generates the Rust module

The proc macros handle everything automatically - no build scripts, no separate crates needed!

## Advanced Features

### Enums with Data

```rust
#[derive(CapnpType)]
pub enum Message {
    Text(#[capnp(id=0)] String),
    Image {
        #[capnp(id=1)]
        url: String,
        #[capnp(id=2)]
        caption: String,
    },
    Video(#[capnp(id=3)] String, #[capnp(id=4)] u32),
}
```

Generates:

```capnp
struct Message {
  union {
    text :group {
      field0 @0 :Text;
    }
    image :group {
      url @1 :Text;
      caption @2 :Text;
    }
    video :group {
      field0 @3 :Text;
      field1 @4 :UInt32;
    }
  }
}
```

### Backwards Compatibility

```rust
#[derive(CapnpType)]
#[capnp(extra="oldUserId @1 :UInt64")]
#[capnp(extra="deprecatedFlag @3 :Bool")]
pub struct UserProfile {
    #[capnp(id=0)]
    pub username: String,
    #[capnp(id=2)]
    pub email: String,
    #[capnp(id=4)]
    pub active: bool,
}
```

Generates:

```capnp
struct UserProfile {
  username @0 :Text;
  email @2 :Text;
  active @4 :Bool;
  oldUserId @1 :UInt64;
  deprecatedFlag @3 :Bool;
}
```

## Traditional Build Script Approach

For cases where you need more control or want to generate schemas separately, you can still use the traditional approach:

```rust
use code_first_capnp_macros::CapnpType;

#[derive(CapnpType)]
pub struct Person {
    #[capnp(id=0)]
    pub id: u64,
    #[capnp(id=1)]
    pub name: String,
}

// In your build script:
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Person::get_capnp_schema();
    let schema_text = code_first_capnp::schema_from_items(&[schema])?;
    // Write to file and use capnpc...
    Ok(())
}
```

## Examples

See the `examples/` directory for complete working examples:

- `examples/demo/` - **New single-crate approach** (recommended)
- `examples/demo-types/` - Comprehensive example showing all features  
- `examples/demo-run/` - Traditional separate-crate approach with build script

## TODO

- Integration with the main capnp library (auto-generate `TryFrom<>` implementations)
- Support for more Cap'n Proto features (interfaces, generics, etc.)
- Support for Cap'n Proto RPC interfaces

## License

This project is licensed under the MIT License.
