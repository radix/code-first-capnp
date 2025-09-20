# Code-First Cap'n Proto for Rust

Generate Cap'n Proto schemas directly from Rust types using proc macros.

## Overview

This library allows you to define your data structures in Rust and automatically generate corresponding Cap'n Proto schemas at compile time. No need to maintain separate `.capnp` files - your Rust types are the source of truth.

## Features

- **Compile-time generation** using proc macros (zero runtime overhead)
- **Automatic field naming** with snake_case to camelCase conversion
- **Manual field IDs** with `#[capnp(id=N)]` attributes
- **Custom field names** with `#[capnp(name="customName")]`
- **Enum support** unit variants become void types, data variants become union groups
- **Backwards compatibility** with `#[capnp(extra="field @id :Type")]` for deprecated fields
- **Schema validation** with duplicate ID detection
- **Type safety** with full Rust type system integration

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
code-first-capnp = "0.1.0"
```

Define your types:

```rust
use code_first_capnp_macros::CapnpType;

#[derive(CapnpType)]
pub struct Person {
    #[capnp(id=0)]
    pub id: u64,

    #[capnp(id=1, name="fullName")]
    pub name: String,

    #[capnp(id=2)]
    pub email_addresses: Vec<String>,

    #[capnp(id=3)]
    pub age: u16,

    #[capnp(id=4)]
    pub is_active: bool,
}

#[derive(CapnpType)]
pub enum Status {
    #[capnp(id=0)]
    Active,
    #[capnp(id=1)]
    Inactive,
    #[capnp(id=2)]
    Pending,
}
```

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

## Examples

See the `examples/` directory for complete working examples:

- `examples/demo-types/` - Comprehensive example showing all features
- `examples/demo-run/` - Integration with capnp-generated Rust code

## TODO

- Integration with the main capnp library (auto-generate `TryFrom<>` implementations)
- Support for more Cap'n Proto features (interfaces, generics, etc.)
- Support for Cap'n Proto RPC interfaces

## License

This project is licensed under the MIT License.
