//! Code-First Cap'n Proto Schema Generation for Rust
//!
//! This library provides tools for generating Cap'n Proto schemas from Rust types
//! using the `facet` crate for type introspection.
//!
//! ## Architecture
//!
//! The library is structured in two main layers:
//!
//! 1. **Document Model** (`capnp_model` module): Provides abstract data structures
//!    that represent Cap'n Proto schemas independent of string generation. This
//!    includes `CapnpDocument`, `CapnpStruct`, `CapnpField`, etc.
//!
//! 2. **Shape Processing** (this module): Contains functions that convert facet
//!    `Shape` objects into the Cap'n Proto document model, then render them as
//!    schema text.
//!
//! ## Usage
//!
//! Use `capnp_schema_for<T>()` as the main entry point - it works for both structs
//! and enums, generating complete schemas.
//!
//! ```rust,ignore
//! // For any type implementing Facet
//! let schema = capnp_schema_for::<MyType>()?;
//! println!("{}", schema);
//! ```
//!
//! ## Enum Handling
//!
//! Enums are rendered as Cap'n Proto structs containing unions. Variants with associated
//! data become **groups** within the union rather than separate struct definitions:
//!
//! ```rust,ignore
//! #[derive(Facet)]
//! enum Message {
//!     #[facet(capnp:id=0)]
//!     Text(String),
//!     #[facet(capnp:id=1)]
//!     Image { url: String, caption: String },
//! }
//! ```
//!
//! Generates this Cap'n Proto schema:
//!
//! ```capnp
//! struct Message {
//!   union {
//!     Text :group @0 {
//!       field0 @0 :Text;
//!     };
//!     Image :group @1 {
//!       url @0 :Text;
//!       caption @1 :Text;
//!     };
//!   }
//! }
//! ```
//!
//! This approach produces cleaner schemas with fewer top-level types compared to
//! generating separate helper structs for each variant.
//!
//! ## Direct Model API Usage
//!
//! You can also work with the document model directly for more control:
//!
//! ```rust,ignore
//! use code_first_capnp::*;
//!
//! // Build the model objects
//! let document = build_capnp_document_from_shape::<MyEnum>()?;
//!
//! // Inspect or modify the model
//! for item in &document.items {
//!     if let CapnpItem::Struct(s) = item {
//!         println!("Found struct: {}", s.name);
//!     }
//! }
//!
//! // Render to string
//! let schema_text = document.render();
//! ```
//!
//! ## Conventions
//!
//! - Put `#[facet(capnp:id=<N>)]` on fields/variants to specify field number (required)
//! - Optionally `#[facet(name=<foo>)]` to rename in the .capnp
//! - Enum unit variants become `Void` types in the union
//! - Enum variants with data become inline groups

use facet::{
    Facet, Field, FieldAttribute, NumericType, PrimitiveType, SequenceType, Shape, ShapeLayout,
    StructKind, TextualType, Type, UserType,
};

mod capnp_model;
pub use capnp_model::*;
/// Generate a Cap'n Proto struct for a Rust struct
pub fn capnp_struct_for<T: Facet<'static>>() -> Result<String, String> {
    let capnp_struct = build_capnp_struct_from_shape::<T>()?;
    let document = CapnpDocument::with_struct(capnp_struct);
    Ok(document.render())
}

/// Generates a complete Cap'n Proto schema for any type.
/// For structs, produces the struct definition.
/// For enums, produces both the union struct and all variant helper structs.
pub fn capnp_schema_for<T: Facet<'static>>() -> Result<String, String> {
    let document = build_capnp_document_from_shape::<T>()?;
    Ok(document.render())
}

/// Generate a Cap'n Proto union for a Rust enum
pub fn capnp_union_for<T: Facet<'static>>() -> Result<String, String> {
    let capnp_struct = build_capnp_union_from_shape::<T>()?;
    let document = CapnpDocument::with_struct(capnp_struct);
    Ok(document.render())
}

fn extract_capnp_id_from_variant_attrs(variant: &facet::Variant) -> Option<u32> {
    for attr in variant.attributes {
        let facet::VariantAttribute::Arbitrary(s) = attr;

        // Parse attributes in the format: 'capnp : id = N'
        if let Some(rest) = s.strip_prefix("capnp : ") {
            if let Some(id_str) = rest.strip_prefix("id = ") {
                if let Ok(n) = id_str.trim().parse::<u32>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

fn capnp_overrides_from_attrs(field: &Field) -> (Option<String>, Option<u32>) {
    let mut name: Option<String> = None;
    let mut id: Option<u32> = None;

    for attr in field.attributes {
        let FieldAttribute::Arbitrary(s) = attr;

        // Parse attributes in the format: 'capnp : id = 0' or 'name = fullName'
        if let Some(rest) = s.strip_prefix("capnp : ") {
            // Handle 'capnp : id = N' format
            if let Some(id_str) = rest.strip_prefix("id = ") {
                if let Ok(n) = id_str.trim().parse::<u32>() {
                    id = Some(n);
                }
            }
        } else if let Some(name_str) = s.strip_prefix("name = ") {
            // Handle 'name = fieldName' format
            let name_value = name_str.trim();
            if !name_value.is_empty() {
                name = Some(name_value.to_string());
            }
        }
    }
    (name, id)
}

/// Minimal mapping from facet `Shape` to Cap'n Proto type tokens.
/// Extend this as you add support (Option<T>, maps/sets, enums/newtypes, etc).
/// Converts a facet Shape to a CapnpType
fn shape_to_capnp_type(shape: &'static Shape) -> Result<CapnpType, String> {
    match shape.ty {
        Type::Primitive(p) => Ok(match p {
            PrimitiveType::Numeric(n) => {
                // Get the size in bytes from the shape's layout
                let layout = match shape.layout {
                    ShapeLayout::Sized(layout) => layout,
                    ShapeLayout::Unsized => {
                        return Err("Cannot handle unsized numeric types".into());
                    }
                };
                let size_bytes = layout.size();

                match n {
                    NumericType::Integer { signed } => match (size_bytes, signed) {
                        (1, false) => CapnpType::UInt8,
                        (2, false) => CapnpType::UInt16,
                        (4, false) => CapnpType::UInt32,
                        (8, false) => CapnpType::UInt64,
                        (16, false) => return Err("UInt128 not supported in Cap'n Proto".into()),
                        (1, true) => CapnpType::Int8,
                        (2, true) => CapnpType::Int16,
                        (4, true) => CapnpType::Int32,
                        (8, true) => CapnpType::Int64,
                        (16, true) => return Err("Int128 not supported in Cap'n Proto".into()),
                        _ => return Err(format!("Unsupported integer size: {} bytes", size_bytes)),
                    },
                    NumericType::Float => match size_bytes {
                        4 => CapnpType::Float32,
                        8 => CapnpType::Float64,
                        _ => return Err(format!("Unsupported float size: {} bytes", size_bytes)),
                    },
                }
            }
            PrimitiveType::Boolean => CapnpType::Bool,
            PrimitiveType::Textual(t) => match t {
                TextualType::Str | TextualType::Char => CapnpType::Text, // store char as 1-char Text for now
            },
            PrimitiveType::Never => {
                return Err("Never type (!) cannot be represented in Cap'n Proto".into());
            }
        }),

        Type::Sequence(seq) => {
            // Handle different sequence types
            let inner_shape = match seq {
                SequenceType::Array(array_type) => array_type.t,
                SequenceType::Slice(slice_type) => slice_type.t,
            };

            let inner_capnp_type = shape_to_capnp_type(inner_shape)?;
            Ok(CapnpType::List(Box::new(inner_capnp_type)))
        }

        Type::User(user_type) => {
            match user_type {
                UserType::Struct(_) => {
                    // Reference by type name — assume you'll emit that struct separately
                    Ok(CapnpType::UserDefined(shape.type_identifier.to_string()))
                }
                UserType::Enum(_) => {
                    // Enums become unions in Cap'n Proto - reference by type name
                    Ok(CapnpType::UserDefined(shape.type_identifier.to_string()))
                }
                UserType::Opaque => {
                    // Handle common opaque types based on their type identifier
                    match shape.type_identifier {
                        "String" => Ok(CapnpType::Text),
                        "Vec" => {
                            // For Vec<T>, we need to look at the type parameters to get T
                            if let Some(type_param) = shape.type_params.first() {
                                let inner_capnp_type = shape_to_capnp_type((type_param.shape)())?;
                                Ok(CapnpType::List(Box::new(inner_capnp_type)))
                            } else {
                                Err("Vec type without type parameter".into())
                            }
                        }
                        _ => Err(format!(
                            "Unsupported opaque type: {}",
                            shape.type_identifier
                        )),
                    }
                }
                UserType::Union(_) => Err("Union types not yet supported".into()),
            }
        }

        Type::Pointer(_) => Err(
            "pointers/smart-pointers not directly supported in Cap'n Proto; wrap/flatten".into(),
        ),
    }
}

/// Builds a CapnpStruct from a facet struct shape
pub fn build_capnp_struct_from_shape<T: Facet<'static>>() -> Result<CapnpStruct, String> {
    let shape = T::SHAPE;

    let (st_name, st) = match shape.ty {
        Type::User(UserType::Struct(sd)) => (shape.type_identifier, sd),
        _ => return Err(format!("{} is not a struct", shape.type_identifier)),
    };

    let mut capnp_struct = CapnpStruct::new(st_name.to_string());

    // Only record/tuple structs are supported here; unit struct becomes empty record.
    if matches!(st.kind, StructKind::Unit) {
        return Ok(capnp_struct);
    }

    // Build fields
    for fld in st.fields.iter() {
        let (name_override, id_override) = capnp_overrides_from_attrs(fld);

        // choose the Cap'n Proto field name
        let capnp_name = name_override.unwrap_or_else(|| fld.name.to_string());

        // Field ID is required
        let id = match id_override {
            Some(n) => n,
            None => {
                return Err(format!(
                    "Field '{}' missing required capnp:id attribute. Use #[facet(capnp:id=N)]",
                    fld.name
                ));
            }
        };

        // Map facet type → Cap'n Proto type
        let capnp_ty = shape_to_capnp_type(fld.shape)?;

        capnp_struct.add_field(CapnpField::new(capnp_name, id, capnp_ty));
    }

    Ok(capnp_struct)
}

/// Builds a CapnpStruct with union from a facet enum shape
pub fn build_capnp_union_from_shape<T: Facet<'static>>() -> Result<CapnpStruct, String> {
    let shape = T::SHAPE;

    let (enum_name, enum_def) = match shape.ty {
        Type::User(UserType::Enum(ed)) => (shape.type_identifier, ed),
        _ => return Err(format!("{} is not an enum", shape.type_identifier)),
    };

    let mut capnp_struct = CapnpStruct::new(enum_name.to_string());
    let mut union = CapnpUnion::new();

    // Build union variants
    for variant in enum_def.variants.iter() {
        let variant_name = variant.name;

        // Extract field ID from variant attributes
        let variant_id = match extract_capnp_id_from_variant_attrs(variant) {
            Some(id) => id,
            None => {
                return Err(format!(
                    "Variant '{}' missing required capnp:id attribute. Use #[facet(capnp:id=N)]",
                    variant_name
                ));
            }
        };

        match variant.data.kind {
            StructKind::Unit => {
                // Unit variants become Void in Cap'n Proto
                union.add_variant(CapnpUnionVariant::new(
                    variant_name.to_string(),
                    variant_id,
                    CapnpType::Void,
                ));
            }
            StructKind::Tuple => {
                // Tuple variants become groups with numbered fields
                if variant.data.fields.is_empty() {
                    union.add_variant(CapnpUnionVariant::new(
                        variant_name.to_string(),
                        variant_id,
                        CapnpType::Void,
                    ));
                } else {
                    let mut group_fields = Vec::new();
                    for (field_idx, field) in variant.data.fields.iter().enumerate() {
                        let field_name = format!("field{}", field_idx);
                        let capnp_ty = shape_to_capnp_type(field.shape)?;
                        group_fields.push(CapnpField::new(field_name, field_idx as u32, capnp_ty));
                    }
                    union.add_variant(CapnpUnionVariant::new_group(
                        variant_name.to_string(),
                        variant_id,
                        group_fields,
                    ));
                }
            }
            StructKind::TupleStruct => {
                // TupleStruct variants become groups with numbered fields
                if variant.data.fields.is_empty() {
                    union.add_variant(CapnpUnionVariant::new(
                        variant_name.to_string(),
                        variant_id,
                        CapnpType::Void,
                    ));
                } else {
                    let mut group_fields = Vec::new();
                    for (field_idx, field) in variant.data.fields.iter().enumerate() {
                        let field_name = format!("field{}", field_idx);
                        let capnp_ty = shape_to_capnp_type(field.shape)?;
                        group_fields.push(CapnpField::new(field_name, field_idx as u32, capnp_ty));
                    }
                    union.add_variant(CapnpUnionVariant::new_group(
                        variant_name.to_string(),
                        variant_id,
                        group_fields,
                    ));
                }
            }
            StructKind::Struct => {
                // Named struct variants become groups with named fields
                let mut group_fields = Vec::new();
                for (field_idx, field) in variant.data.fields.iter().enumerate() {
                    let field_name = field.name.to_string();
                    let capnp_ty = shape_to_capnp_type(field.shape)?;
                    group_fields.push(CapnpField::new(field_name, field_idx as u32, capnp_ty));
                }
                union.add_variant(CapnpUnionVariant::new_group(
                    variant_name.to_string(),
                    variant_id,
                    group_fields,
                ));
            }
        };
    }

    capnp_struct.set_union(union);
    Ok(capnp_struct)
}

/// Builds a complete CapnpDocument for any supported type
pub fn build_capnp_document_from_shape<T: Facet<'static>>() -> Result<CapnpDocument, String> {
    let shape = T::SHAPE;
    let mut document = CapnpDocument::new();

    match shape.ty {
        Type::User(UserType::Struct(_)) => {
            // For structs, just generate the struct definition
            let capnp_struct = build_capnp_struct_from_shape::<T>()?;
            document.add_item(CapnpItem::Struct(capnp_struct));
        }
        Type::User(UserType::Enum(_)) => {
            // For enums, generate only the union struct with groups
            let union_struct = build_capnp_union_from_shape::<T>()?;
            document.add_item(CapnpItem::Struct(union_struct));
        }
        _ => {
            return Err(format!(
                "{} is not a supported type (must be struct or enum)",
                shape.type_identifier
            ));
        }
    }

    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use facet::Facet;

    #[derive(Facet)]
    struct TestStruct {
        #[facet(capnp:id=0)]
        id: u64,
        #[facet(capnp:id=1,name=fullName)]
        name: String,
        #[facet(capnp:id=2)]
        numbers: Vec<i32>,
        #[facet(capnp:id=3)]
        active: bool,
    }

    #[test]
    fn test_basic_struct_model() {
        let capnp_struct = build_capnp_struct_from_shape::<TestStruct>().unwrap();

        assert_eq!(capnp_struct.name, "TestStruct");
        assert_eq!(capnp_struct.fields.len(), 4);
        assert!(capnp_struct.union.is_none());

        // Check specific fields and their types
        let id_field = &capnp_struct.fields[0];
        assert_eq!(id_field.name, "id");
        assert_eq!(id_field.id, 0);
        assert_eq!(id_field.field_type, CapnpType::UInt64);

        let name_field = &capnp_struct.fields[1];
        assert_eq!(name_field.name, "fullName"); // Should use name override
        assert_eq!(name_field.id, 1);
        assert_eq!(name_field.field_type, CapnpType::Text);

        let numbers_field = &capnp_struct.fields[2];
        assert_eq!(numbers_field.name, "numbers");
        assert_eq!(numbers_field.id, 2);
        assert_eq!(
            numbers_field.field_type,
            CapnpType::List(Box::new(CapnpType::Int32))
        );

        let active_field = &capnp_struct.fields[3];
        assert_eq!(active_field.name, "active");
        assert_eq!(active_field.id, 3);
        assert_eq!(active_field.field_type, CapnpType::Bool);
    }

    #[derive(Facet)]
    struct EmptyStruct;

    #[test]
    fn test_unit_struct_model() {
        let capnp_struct = build_capnp_struct_from_shape::<EmptyStruct>().unwrap();

        assert_eq!(capnp_struct.name, "EmptyStruct");
        assert_eq!(capnp_struct.fields.len(), 0);
        assert!(capnp_struct.union.is_none());
    }

    #[derive(Facet)]
    struct MissingIdStruct {
        #[facet(capnp:id=0)]
        id: u64,
        // This field is missing the required capnp:id attribute
        name: String,
    }

    #[test]
    fn test_missing_id_error() {
        let result = build_capnp_struct_from_shape::<MissingIdStruct>();
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Field 'name' missing required capnp:id attribute"));
        assert!(error_msg.contains("#[facet(capnp:id=N)]"));
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
    }

    #[derive(Facet)]
    #[repr(u8)]
    enum ComplexEnum {
        #[facet(capnp:id=0)]
        Unit,
        #[facet(capnp:id=1)]
        Tuple(u32, String),
        #[facet(capnp:id=2)]
        Struct { id: u64, name: String },
    }

    #[test]
    fn test_simple_enum_union_model() {
        let capnp_union = build_capnp_union_from_shape::<Status>().unwrap();

        assert_eq!(capnp_union.name, "Status");
        assert!(capnp_union.fields.is_empty()); // No top-level fields, just union
        assert!(capnp_union.union.is_some());

        let union = capnp_union.union.unwrap();
        assert_eq!(union.variants.len(), 3);

        // Check each variant
        let active_variant = &union.variants[0];
        assert_eq!(active_variant.name, "Active");
        assert_eq!(active_variant.id, 0);
        assert!(matches!(
            active_variant.variant_type,
            CapnpVariantType::Type(CapnpType::Void)
        ));

        let inactive_variant = &union.variants[1];
        assert_eq!(inactive_variant.name, "Inactive");
        assert_eq!(inactive_variant.id, 1);
        assert!(matches!(
            inactive_variant.variant_type,
            CapnpVariantType::Type(CapnpType::Void)
        ));

        let pending_variant = &union.variants[2];
        assert_eq!(pending_variant.name, "Pending");
        assert_eq!(pending_variant.id, 2);
        assert!(matches!(
            pending_variant.variant_type,
            CapnpVariantType::Type(CapnpType::Void)
        ));
    }

    #[test]
    fn test_complex_enum_union_model() {
        let union_struct = build_capnp_union_from_shape::<ComplexEnum>().unwrap();

        assert_eq!(union_struct.name, "ComplexEnum");
        assert!(union_struct.fields.is_empty());
        assert!(union_struct.union.is_some());

        let union = union_struct.union.unwrap();
        assert_eq!(union.variants.len(), 3);

        let unit_variant = &union.variants[0];
        assert_eq!(unit_variant.name, "Unit");
        assert_eq!(unit_variant.id, 0);
        assert!(matches!(
            unit_variant.variant_type,
            CapnpVariantType::Type(CapnpType::Void)
        ));

        let tuple_variant = &union.variants[1];
        assert_eq!(tuple_variant.name, "Tuple");
        assert_eq!(tuple_variant.id, 1);
        if let CapnpVariantType::Group(fields) = &tuple_variant.variant_type {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "field0");
            assert_eq!(fields[0].id, 0);
            assert_eq!(fields[0].field_type, CapnpType::UInt32);
            assert_eq!(fields[1].name, "field1");
            assert_eq!(fields[1].id, 1);
            assert_eq!(fields[1].field_type, CapnpType::Text);
        } else {
            panic!("Expected Group variant type");
        }

        let struct_variant = &union.variants[2];
        assert_eq!(struct_variant.name, "Struct");
        assert_eq!(struct_variant.id, 2);
        if let CapnpVariantType::Group(fields) = &struct_variant.variant_type {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "id");
            assert_eq!(fields[0].id, 0);
            assert_eq!(fields[0].field_type, CapnpType::UInt64);
            assert_eq!(fields[1].name, "name");
            assert_eq!(fields[1].id, 1);
            assert_eq!(fields[1].field_type, CapnpType::Text);
        } else {
            panic!("Expected Group variant type");
        }
    }

    #[derive(Facet)]
    #[repr(u8)]
    enum MissingIdEnum {
        #[facet(capnp:id=0)]
        HasId,
        // This variant is missing the required capnp:id attribute
        MissingId,
    }

    #[test]
    fn test_missing_variant_id_error() {
        let result = build_capnp_union_from_shape::<MissingIdEnum>();
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Variant 'MissingId' missing required capnp:id attribute"));
        assert!(error_msg.contains("#[facet(capnp:id=N)]"));
    }

    #[test]
    fn test_unified_struct_document_model() {
        let document = build_capnp_document_from_shape::<TestStruct>().unwrap();

        assert_eq!(document.items.len(), 1);

        let CapnpItem::Struct(struct_item) = &document.items[0];
        assert_eq!(struct_item.name, "TestStruct");
        assert_eq!(struct_item.fields.len(), 4);
        assert!(struct_item.union.is_none());
    }

    #[test]
    fn test_unified_complex_enum_document_model() {
        let document = build_capnp_document_from_shape::<ComplexEnum>().unwrap();

        assert_eq!(document.items.len(), 1); // Only union struct with groups

        // First item should be the main union struct
        let CapnpItem::Struct(main_struct) = &document.items[0];
        assert_eq!(main_struct.name, "ComplexEnum");
        assert!(main_struct.union.is_some());
        assert!(main_struct.fields.is_empty());

        let union = main_struct.union.as_ref().unwrap();
        assert_eq!(union.variants.len(), 3);

        // Check that variants are groups instead of separate structs
        let unit_variant = &union.variants[0];
        assert_eq!(unit_variant.name, "Unit");
        assert!(matches!(
            unit_variant.variant_type,
            CapnpVariantType::Type(CapnpType::Void)
        ));

        let tuple_variant = &union.variants[1];
        assert_eq!(tuple_variant.name, "Tuple");
        if let CapnpVariantType::Group(fields) = &tuple_variant.variant_type {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "field0");
            assert_eq!(fields[1].name, "field1");
        } else {
            panic!("Expected Group variant type for Tuple");
        }

        let struct_variant = &union.variants[2];
        assert_eq!(struct_variant.name, "Struct");
        if let CapnpVariantType::Group(fields) = &struct_variant.variant_type {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "id");
            assert_eq!(fields[1].name, "name");
        } else {
            panic!("Expected Group variant type for Struct");
        }
    }

    #[test]
    fn test_unified_simple_enum_document_model() {
        let document = build_capnp_document_from_shape::<Status>().unwrap();

        assert_eq!(document.items.len(), 1); // Only union struct, no variant structs needed

        let CapnpItem::Struct(union_struct) = &document.items[0];
        assert_eq!(union_struct.name, "Status");
        assert!(union_struct.fields.is_empty());
        assert!(union_struct.union.is_some());

        let union = union_struct.union.as_ref().unwrap();
        assert_eq!(union.variants.len(), 3);

        // All variants should be Void type
        for variant in &union.variants {
            assert!(matches!(
                variant.variant_type,
                CapnpVariantType::Type(CapnpType::Void)
            ));
        }
    }

    #[test]
    fn test_field_sorting() {
        // Test that fields are sorted by ID
        let capnp_struct = build_capnp_struct_from_shape::<TestStruct>().unwrap();

        let mut prev_id = None;
        for field in &capnp_struct.fields {
            if let Some(prev) = prev_id {
                assert!(field.id > prev, "Fields should be sorted by ID");
            }
            prev_id = Some(field.id);
        }
    }

    #[test]
    fn test_type_mapping_coverage() {
        let capnp_struct = build_capnp_struct_from_shape::<TestStruct>().unwrap();

        // Test various type mappings are covered
        let field_types: Vec<&CapnpType> =
            capnp_struct.fields.iter().map(|f| &f.field_type).collect();

        assert!(field_types.contains(&&CapnpType::UInt64));
        assert!(field_types.contains(&&CapnpType::Text));
        assert!(field_types.contains(&&CapnpType::Bool));
        assert!(field_types.contains(&&CapnpType::List(Box::new(CapnpType::Int32))));
    }
}
