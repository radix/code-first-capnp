//! Code-First Cap'n Proto Schema Generation for Rust
//!
//! This library provides tools for generating Cap'n Proto schemas from Rust types
//! using proc macros for compile-time code generation.
//!
//! ## Architecture
//!
//! The library is structured in two main layers:
//!
//! 1. **Document Model** (`capnp_model` module): Provides abstract data structures
//!    that represent Cap'n Proto schemas independent of string generation. This
//!    includes `Schema`, `Struct`, `Field`, etc.
//!
//! 2. **Proc Macro** (`code-first-capnp-macros` crate): A derive macro that generates
//!    `get_capnp_schema()` methods returning `SchemaItem` instances.
//!
//! ## Usage
//!
//! Use the `#[derive(CapnpType)]` macro on your types and then call `get_capnp_schema()`:
//!
//! ```rust,ignore
//! use code_first_capnp_macros::CapnpType;
//!
//! #[derive(CapnpType)]
//! struct Person {
//!     #[capnp(id=0)]
//!     name: String,
//!     #[capnp(id=1)]
//!     age: u16,
//! }
//!
//! let schema = Person::get_capnp_schema();
//! let schema_text = schema_from_items(&[schema])?;
//! ```
//!
//! ## Enum Handling
//!
//! Enums are rendered as Cap'n Proto structs containing unions. Variants with associated
//! data become **groups** within the union rather than separate struct definitions.

pub use capnp_model::{
    CapnpType, Field as CapnpField, Schema, SchemaItem, Struct, Union, UnionVariant,
    UnionVariantInner,
};

// Re-export the proc macros
pub use code_first_capnp_macros::{CapnpType, capnp_schema_file, complete_capnp_schema};

/// Builds a complete Cap'n Proto schema file with the given ID and schema items
pub fn build_capnp_file(
    file_id: u64,
    items: &[SchemaItem],
) -> Result<String, Box<dyn std::error::Error>> {
    let mut schema = Schema::new();
    for item in items {
        schema.add_item(item.clone());
    }

    // Validate the schema for ID conflicts
    schema.validate()?;

    // Render the schema with file ID
    let schema_content = schema.render()?;
    Ok(format!("@0x{:x};\n\n{}", file_id, schema_content))
}

/// Generates a Cap'n Proto schema from a collection of schema items
pub fn schema_from_items(items: &[SchemaItem]) -> Result<String, Box<dyn std::error::Error>> {
    let mut schema = Schema::new();
    for item in items {
        schema.add_item(item.clone());
    }

    // Validate the schema for ID conflicts
    schema.validate()?;

    // Render the schema
    schema.render().map_err(|e| e.into())
}

/// Generates a Cap'n Proto schema for a single item
pub fn schema_for_item(item: &SchemaItem) -> Result<String, Box<dyn std::error::Error>> {
    schema_from_items(&[item.clone()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(CapnpType)]
    #[allow(dead_code)]
    struct TestStruct {
        #[capnp(id = 0)]
        id: u64,
        #[capnp(id = 1, name = "fullName")]
        name: String,
        #[capnp(id = 2)]
        numbers: Vec<u32>,
        #[capnp(id = 3)]
        active: bool,
    }

    #[test]
    fn test_basic_struct_generation() {
        let schema = TestStruct::get_capnp_schema();
        let schema_text = schema_for_item(&schema).unwrap();

        // Basic sanity checks
        assert!(schema_text.contains("struct TestStruct"));
        assert!(schema_text.contains("id @0 :UInt64"));
        assert!(schema_text.contains("fullName @1 :Text"));
        assert!(schema_text.contains("numbers @2 :List(UInt32)"));
        assert!(schema_text.contains("active @3 :Bool"));
    }

    #[derive(CapnpType)]
    #[allow(dead_code)]
    struct EmptyStruct;

    #[test]
    fn test_unit_struct() {
        let schema = EmptyStruct::get_capnp_schema();
        let schema_text = schema_for_item(&schema).unwrap();
        assert!(schema_text.contains("struct EmptyStruct"));
    }
}
