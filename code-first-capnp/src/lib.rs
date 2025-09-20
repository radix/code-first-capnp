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
