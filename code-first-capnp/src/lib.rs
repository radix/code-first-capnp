//! Code-First Cap'n Proto Schema Generation for Rust
//!
//! This library provides tools for generating Cap'n Proto schemas from Rust types
//! using proc macros for compile-time code generation.
//!
//! ## Two Usage Approaches
//!
//! ### 1. New Single-Crate Approach (Recommended)
//!
//! Define types and use generated capnp code in the same crate:
//!
//! ```rust,ignore
//! use code_first_capnp_macros::CapnpType;
//! use code_first_capnp::{capnp_schema_file, complete_capnp_schema};
//!
//! // Initialize the schema file
//! capnp_schema_file!("demo.capnp", 0xfbb45a811fbe71f5);
//!
//! #[derive(CapnpType)]
//! #[capnp(file = "demo.capnp")]
//! struct Person {
//!     #[capnp(id = 0)]
//!     id: u64,
//!     #[capnp(id = 1, name = "fullName")]
//!     name: String,
//! }
//!
//! #[derive(CapnpType)]
//! #[capnp(file = "demo.capnp")]
//! enum Status {
//!     #[capnp(id = 0)]
//!     Active,
//!     #[capnp(id = 1)]
//!     Inactive,
//! }
//!
//! // Complete the schema and generate the capnp module
//! complete_capnp_schema!("demo.capnp", pub mod demo_capnp);
//!
//! // Now you can use the generated types:
//! fn example() -> capnp::Result<()> {
//!     let mut message = capnp::message::Builder::new_default();
//!     let mut person = message.init_root::<demo_capnp::person::Builder>();
//!     person.set_id(123);
//!     person.set_full_name("John Doe");
//!     Ok(())
//! }
//! ```
//!
//! ### 2. Traditional Build Script Approach
//!
//! Use `#[derive(CapnpType)]` and call `get_capnp_schema()` in build scripts:
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
//! ## Architecture
//!
//! The library consists of:
//!
//! 1. **`capnp-model` crate**: Abstract data structures for Cap'n Proto schemas
//! 2. **`code-first-capnp-macros` crate**: Proc macros for schema generation
//! 3. **`code-first-capnp` crate**: Main library that re-exports everything
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
