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
//! ## Backwards Compatibility with Extra Fields
//!
//! Cap'n Proto requires that field IDs never have gaps and that deleted fields remain
//! in the schema for backwards compatibility. To support this, you can use the
//! `#[facet(capnp:extra="...")]` attribute on structs and enums to include
//! deleted/deprecated fields in the generated schema:
//!
//! ```rust,ignore
//! #[derive(Facet)]
//! #[facet(capnp:extra="oldUserId @1 :UInt64")]
//! #[facet(capnp:extra="deprecatedFlag @3 :Bool")]
//! struct UserProfile {
//!     #[facet(capnp:id=0)]
//!     username: String,
//!     #[facet(capnp:id=2)]
//!     email: String,
//!     #[facet(capnp:id=4)]
//!     active: bool,
//! }
//! ```
//!
//! This generates:
//!
//! ```capnp
//! struct UserProfile {
//!   username @0 :Text;
//!   email @2 :Text;
//!   active @4 :Bool;
//!   oldUserId @1 :UInt64;
//!   deprecatedFlag @3 :Bool;
//! }
//! ```
//!
//! This way you can remove fields from your Rust code while maintaining Cap'n Proto
//! schema compatibility.
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
//! // Render to string (with validation)
//! let schema_text = document.render().unwrap();
//! ```
//!
//! ## Conventions
//!
//! - Put `#[facet(capnp:id=<N>)]` on fields/variants to specify field number (required)
//! - Optionally `#[facet(name=<foo>)]` to rename in the .capnp
//! - Use `#[facet(capnp:extra="fieldName @N :Type")]` on types for backwards compatibility
//! - Enum unit variants become `Void` types in the union
//! - Enum variants with data become inline groups

use facet::{
    Facet, Field, FieldAttribute, NumericType, PrimitiveType, SequenceType, Shape, ShapeAttribute,
    ShapeLayout, StructKind, TextualType, Type, UserType,
};
use heck::ToLowerCamelCase;

mod capnp_model;
pub use capnp_model::{
    CapnpType, Field as CapnpField, Schema, SchemaItem, Struct, Union, UnionVariant,
    UnionVariantInner,
};

/// Builds a complete Cap'n Proto schema file with the given ID and shapes
pub fn build_capnp_file(id: u64, shapes: &[&'static Shape]) -> Result<String, String> {
    let mut output = String::new();

    // Add file ID in hexadecimal at the top
    output.push_str(&format!("@0x{:x};\n\n", id));

    let mut document = Schema::new();

    // Process each shape and add to document
    for shape in shapes {
        match shape.ty {
            Type::User(UserType::Struct(_)) => {
                let capnp_struct = build_capnp_struct_from_shape(shape)?;
                document.add_item(SchemaItem::Struct(capnp_struct));
            }
            Type::User(UserType::Enum(_)) => {
                let union_struct = build_capnp_union_from_shape(shape)?;
                document.add_item(SchemaItem::Struct(union_struct));
            }
            _ => {
                return Err(format!(
                    "{} is not a supported type (must be struct or enum)",
                    shape.type_identifier
                ));
            }
        }
    }

    let schema_body = document.render().map_err(|e| e.to_string())?;
    output.push_str(&schema_body);

    Ok(output)
}

/// Generate a Cap'n Proto struct for a Rust struct
pub fn capnp_struct_for<T: Facet<'static>>() -> Result<String, String> {
    let capnp_struct = build_capnp_struct_from_shape(T::SHAPE)?;
    let document = Schema::with_struct(capnp_struct);
    document.render().map_err(|e| e.to_string())
}

/// Generates a complete Cap'n Proto schema for any type.
/// For structs, produces the struct definition.
/// For enums, produces both the union struct and all variant helper structs.
pub fn capnp_schema_for<T: Facet<'static>>() -> Result<String, String> {
    let document = build_capnp_document_from_shape(T::SHAPE)?;
    document.render().map_err(|e| e.to_string())
}

/// Generate a Cap'n Proto union for a Rust enum
pub fn capnp_union_for<T: Facet<'static>>() -> Result<String, String> {
    let capnp_struct = build_capnp_union_from_shape(T::SHAPE)?;
    let document = Schema::with_struct(capnp_struct);
    document.render().map_err(|e| e.to_string())
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

fn variant_has_data(variant: &facet::Variant) -> bool {
    match variant.data.kind {
        StructKind::Unit => false,
        _ => !variant.data.fields.is_empty(),
    }
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

fn extract_capnp_extra_fields_from_shape_attrs(shape: &'static Shape) -> Vec<String> {
    let mut extra_fields = Vec::new();

    for attr in shape.attributes {
        if let ShapeAttribute::Arbitrary(s) = attr {
            // Parse attributes in the format: 'capnp : extra = "fieldName @3 :UInt64"'
            if let Some(rest) = s.strip_prefix("capnp : extra = ") {
                // Remove surrounding quotes if present
                let extra_value = rest.trim().trim_matches('"');
                if !extra_value.is_empty() {
                    extra_fields.push(extra_value.to_string());
                }
            }
        }
    }

    extra_fields
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
pub fn build_capnp_struct_from_shape(shape: &'static Shape) -> Result<Struct, String> {

    let (st_name, st) = match shape.ty {
        Type::User(UserType::Struct(sd)) => (shape.type_identifier, sd),
        _ => return Err(format!("{} is not a struct", shape.type_identifier)),
    };

    let mut capnp_struct = Struct::new(st_name.to_string());

    // Extract extra fields from struct-level attributes
    let extra_fields = extract_capnp_extra_fields_from_shape_attrs(shape);
    for extra_field in extra_fields {
        capnp_struct.add_extra_field(extra_field);
    }

    // Only record/tuple structs are supported here; unit struct becomes empty record.
    if matches!(st.kind, StructKind::Unit) {
        return Ok(capnp_struct);
    }

    // Build fields
    for fld in st.fields.iter() {
        let (name_override, id_override) = capnp_overrides_from_attrs(fld);

        // choose the Cap'n Proto field name (convert to camelCase)
        let capnp_name = name_override.unwrap_or_else(|| fld.name.to_lower_camel_case());

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
pub fn build_capnp_union_from_shape(shape: &'static Shape) -> Result<Struct, String> {

    let (enum_name, enum_def) = match shape.ty {
        Type::User(UserType::Enum(ed)) => (shape.type_identifier, ed),
        _ => return Err(format!("{} is not an enum", shape.type_identifier)),
    };

    let mut capnp_struct = Struct::new(enum_name.to_string());
    let mut union = Union::new();

    // Extract extra fields from enum-level attributes
    let extra_fields = extract_capnp_extra_fields_from_shape_attrs(shape);
    for extra_field in extra_fields {
        capnp_struct.add_extra_field(extra_field);
    }

    // Build union variants
    for variant in enum_def.variants.iter() {
        let variant_name = variant.name.to_lower_camel_case();
        let has_data = variant_has_data(variant);
        let variant_id_opt = extract_capnp_id_from_variant_attrs(variant);

        // Validate ID requirements based on variant type
        match (has_data, variant_id_opt) {
            (false, None) => {
                return Err(format!(
                    "Unit variant '{}' missing required capnp:id attribute. Use #[facet(capnp:id=N)]",
                    variant.name
                ));
            }
            (true, Some(_)) => {
                return Err(format!(
                    "Data-bearing variant '{}' should not have capnp:id attribute. Only put IDs on the fields.",
                    variant.name
                ));
            }
            _ => {} // Valid cases: unit with ID, or data-bearing without ID
        }

        match variant.data.kind {
            StructKind::Unit => {
                // Unit variants become Void in Cap'n Proto with their ID
                let variant_id = variant_id_opt.unwrap(); // We validated this exists above
                union.add_variant(UnionVariant::new(
                    variant_name.to_string(),
                    variant_id,
                    CapnpType::Void,
                ));
            }
            StructKind::Tuple | StructKind::TupleStruct => {
                // Tuple/TupleStruct variants become groups with numbered fields
                if variant.data.fields.is_empty() {
                    // Empty tuple struct variant - should have been caught as unit variant
                    let variant_id = variant_id_opt.unwrap();
                    union.add_variant(UnionVariant::new(
                        variant_name.to_string(),
                        variant_id,
                        CapnpType::Void,
                    ));
                } else {
                    let mut group_fields = Vec::new();
                    for field in variant.data.fields.iter() {
                        let (name_override, id_override) = capnp_overrides_from_attrs(field);

                        let field_id = match id_override {
                            Some(id) => id,
                            None => {
                                return Err(format!(
                                    "Field in variant '{}' missing required capnp:id attribute. Use #[facet(capnp:id=N)]",
                                    variant_name
                                ));
                            }
                        };

                        let field_name = name_override.unwrap_or_else(|| {
                            // For tuple variants, field names are numeric ("0", "1", etc.)
                            // Convert these to proper field names
                            format!("field{field_id}")
                        });

                        let capnp_ty = shape_to_capnp_type(field.shape)?;
                        group_fields.push(CapnpField::new(field_name, field_id, capnp_ty));
                    }
                    // Data-bearing variants become groups without variant IDs
                    union.add_variant(UnionVariant::new_group(
                        variant_name.to_string(),
                        group_fields,
                    ));
                }
            }
            StructKind::Struct => {
                // Named struct variants become groups with named fields
                let mut group_fields = Vec::new();
                for field in variant.data.fields.iter() {
                    let (name_override, id_override) = capnp_overrides_from_attrs(field);

                    let field_id = match id_override {
                        Some(id) => id,
                        None => {
                            return Err(format!(
                                "Field '{}' in variant '{}' missing required capnp:id attribute. Use #[facet(capnp:id=N)]",
                                field.name, variant_name
                            ));
                        }
                    };

                    let field_name =
                        name_override.unwrap_or_else(|| field.name.to_lower_camel_case());

                    let capnp_ty = shape_to_capnp_type(field.shape)?;
                    group_fields.push(CapnpField::new(field_name, field_id, capnp_ty));
                }
                // Data-bearing variants become groups without variant IDs
                union.add_variant(UnionVariant::new_group(
                    variant_name.to_string(),
                    group_fields,
                ));
            }
        };
    }

    capnp_struct.set_union(union);
    Ok(capnp_struct)
}

/// Builds a complete CapnpDocument for any supported type
pub fn build_capnp_document_from_shape(shape: &'static Shape) -> Result<Schema, String> {
    let mut document = Schema::new();

    match shape.ty {
        Type::User(UserType::Struct(_)) => {
            // For structs, just generate the struct definition
            let capnp_struct = build_capnp_struct_from_shape(shape)?;
            document.add_item(SchemaItem::Struct(capnp_struct));
        }
        Type::User(UserType::Enum(_)) => {
            // For enums, generate only the union struct with groups
            let union_struct = build_capnp_union_from_shape(shape)?;
            document.add_item(SchemaItem::Struct(union_struct));
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
        let capnp_struct = build_capnp_struct_from_shape(TestStruct::SHAPE).unwrap();

        let expected = Struct {
            name: "TestStruct".to_string(),
            fields: vec![
                CapnpField {
                    name: "id".to_string(),
                    id: 0,
                    field_type: CapnpType::UInt64,
                },
                CapnpField {
                    name: "fullName".to_string(),
                    id: 1,
                    field_type: CapnpType::Text,
                },
                CapnpField {
                    name: "numbers".to_string(),
                    id: 2,
                    field_type: CapnpType::List(Box::new(CapnpType::Int32)),
                },
                CapnpField {
                    name: "active".to_string(),
                    id: 3,
                    field_type: CapnpType::Bool,
                },
            ],
            union: None,
            extra_fields: vec![],
        };

        assert_eq!(capnp_struct, expected);
    }

    #[derive(Facet)]
    struct EmptyStruct;

    #[test]
    fn test_unit_struct_model() {
        let capnp_struct = build_capnp_struct_from_shape(EmptyStruct::SHAPE).unwrap();

        let expected = Struct {
            name: "EmptyStruct".to_string(),
            fields: vec![],
            union: None,
            extra_fields: vec![],
        };

        assert_eq!(capnp_struct, expected);
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
        let result = build_capnp_struct_from_shape(MissingIdStruct::SHAPE);
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert_eq!(
            error_msg,
            "Field 'name' missing required capnp:id attribute. Use #[facet(capnp:id=N)]"
        );
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
    }

    #[allow(dead_code)]
    #[derive(Facet)]
    #[repr(u8)]
    enum ComplexEnum {
        #[facet(capnp:id=0)]
        Unit,
        Tuple(#[facet(capnp:id=10)] u32, #[facet(capnp:id=11)] String),
        Struct {
            #[facet(capnp:id=20)]
            id: u64,
            #[facet(capnp:id=21)]
            name: String,
        },
    }

    #[test]
    fn test_simple_enum_union_model() {
        let capnp_union = build_capnp_union_from_shape(Status::SHAPE).unwrap();

        let expected = Struct {
            name: "Status".to_string(),
            fields: vec![],
            union: Some(Union {
                variants: vec![
                    UnionVariant {
                        name: "active".to_string(),
                        variant_inner: UnionVariantInner::Type {
                            id: 0,
                            capnp_type: CapnpType::Void,
                        },
                    },
                    UnionVariant {
                        name: "inactive".to_string(),
                        variant_inner: UnionVariantInner::Type {
                            id: 1,
                            capnp_type: CapnpType::Void,
                        },
                    },
                    UnionVariant {
                        name: "pending".to_string(),
                        variant_inner: UnionVariantInner::Type {
                            id: 2,
                            capnp_type: CapnpType::Void,
                        },
                    },
                ],
            }),
            extra_fields: vec![],
        };

        assert_eq!(capnp_union, expected);
    }

    #[test]
    fn test_complex_enum_union_model() {
        let union_struct = build_capnp_union_from_shape(ComplexEnum::SHAPE).unwrap();

        let expected = Struct {
            name: "ComplexEnum".to_string(),
            fields: vec![],
            union: Some(Union {
                variants: vec![
                    UnionVariant {
                        name: "unit".to_string(),
                        variant_inner: UnionVariantInner::Type {
                            id: 0,
                            capnp_type: CapnpType::Void,
                        },
                    },
                    UnionVariant {
                        name: "tuple".to_string(),
                        variant_inner: UnionVariantInner::Group(vec![
                            CapnpField {
                                name: "field10".to_string(),
                                id: 10,
                                field_type: CapnpType::UInt32,
                            },
                            CapnpField {
                                name: "field11".to_string(),
                                id: 11,
                                field_type: CapnpType::Text,
                            },
                        ]),
                    },
                    UnionVariant {
                        name: "struct".to_string(),
                        variant_inner: UnionVariantInner::Group(vec![
                            CapnpField {
                                name: "id".to_string(),
                                id: 20,
                                field_type: CapnpType::UInt64,
                            },
                            CapnpField {
                                name: "name".to_string(),
                                id: 21,
                                field_type: CapnpType::Text,
                            },
                        ]),
                    },
                ],
            }),
            extra_fields: vec![],
        };

        assert_eq!(union_struct, expected);
    }

    #[allow(dead_code)]
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
        let result = build_capnp_union_from_shape(MissingIdEnum::SHAPE);
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert_eq!(
            error_msg,
            "Unit variant 'MissingId' missing required capnp:id attribute. Use #[facet(capnp:id=N)]"
        );
    }

    #[test]
    fn test_unified_struct_document_model() {
        let document = build_capnp_document_from_shape(TestStruct::SHAPE).unwrap();

        let expected = Schema {
            items: vec![SchemaItem::Struct(Struct {
                name: "TestStruct".to_string(),
                fields: vec![
                    CapnpField {
                        name: "id".to_string(),
                        id: 0,
                        field_type: CapnpType::UInt64,
                    },
                    CapnpField {
                        name: "fullName".to_string(),
                        id: 1,
                        field_type: CapnpType::Text,
                    },
                    CapnpField {
                        name: "numbers".to_string(),
                        id: 2,
                        field_type: CapnpType::List(Box::new(CapnpType::Int32)),
                    },
                    CapnpField {
                        name: "active".to_string(),
                        id: 3,
                        field_type: CapnpType::Bool,
                    },
                ],
                union: None,
                extra_fields: vec![],
            })],
        };

        assert_eq!(document, expected);
    }

    #[test]
    fn test_unified_complex_enum_document_model() {
        let document = build_capnp_document_from_shape(ComplexEnum::SHAPE).unwrap();
        let expected = Schema {
            items: vec![SchemaItem::Struct(Struct {
                name: "ComplexEnum".to_string(),
                fields: vec![],
                union: Some(Union {
                    variants: vec![
                        UnionVariant {
                            name: "unit".to_string(),
                            variant_inner: UnionVariantInner::Type {
                                id: 0,
                                capnp_type: CapnpType::Void,
                            },
                        },
                        UnionVariant {
                            name: "tuple".to_string(),
                            variant_inner: UnionVariantInner::Group(vec![
                                CapnpField {
                                    name: "field10".to_string(),
                                    id: 10,
                                    field_type: CapnpType::UInt32,
                                },
                                CapnpField {
                                    name: "field11".to_string(),
                                    id: 11,
                                    field_type: CapnpType::Text,
                                },
                            ]),
                        },
                        UnionVariant {
                            name: "struct".to_string(),
                            variant_inner: UnionVariantInner::Group(vec![
                                CapnpField {
                                    name: "id".to_string(),
                                    id: 20,
                                    field_type: CapnpType::UInt64,
                                },
                                CapnpField {
                                    name: "name".to_string(),
                                    id: 21,
                                    field_type: CapnpType::Text,
                                },
                            ]),
                        },
                    ],
                }),
                extra_fields: vec![],
            })],
        };
        assert_eq!(document, expected);
    }

    #[test]
    fn test_unified_simple_enum_document_model() {
        let schema = build_capnp_document_from_shape(Status::SHAPE).unwrap();
        let expected = Schema {
            items: vec![SchemaItem::Struct(Struct {
                name: "Status".to_string(),
                fields: vec![],
                union: Some(Union {
                    variants: vec![
                        UnionVariant {
                            name: "active".to_string(),
                            variant_inner: UnionVariantInner::Type {
                                id: 0,
                                capnp_type: CapnpType::Void,
                            },
                        },
                        UnionVariant {
                            name: "inactive".to_string(),
                            variant_inner: UnionVariantInner::Type {
                                id: 1,
                                capnp_type: CapnpType::Void,
                            },
                        },
                        UnionVariant {
                            name: "pending".to_string(),
                            variant_inner: UnionVariantInner::Type {
                                id: 2,
                                capnp_type: CapnpType::Void,
                            },
                        },
                    ],
                }),
                extra_fields: vec![],
            })],
        };
        assert_eq!(schema, expected);
    }

    #[test]
    fn test_field_sorting() {
        // Test that fields are sorted by ID
        let capnp_struct = build_capnp_struct_from_shape(TestStruct::SHAPE).unwrap();

        let field_ids: Vec<u32> = capnp_struct.fields.iter().map(|f| f.id).collect();
        let expected_ids = vec![0, 1, 2, 3];

        assert_eq!(field_ids, expected_ids);
    }

    #[test]
    fn test_type_mapping_coverage() {
        let capnp_struct = build_capnp_struct_from_shape(TestStruct::SHAPE).unwrap();

        // Test various type mappings are covered
        let field_types: Vec<CapnpType> = capnp_struct
            .fields
            .iter()
            .map(|f| f.field_type.clone())
            .collect();

        let expected_types = vec![
            CapnpType::UInt64,
            CapnpType::Text,
            CapnpType::List(Box::new(CapnpType::Int32)),
            CapnpType::Bool,
        ];

        assert_eq!(field_types, expected_types);
    }

    // Test enum with explicit field IDs like in demo
    #[allow(dead_code)]
    #[derive(Facet)]
    #[repr(u8)]
    enum EnumWithData {
        MyText(#[facet(capnp:id=1)] String),
        Image {
            #[facet(capnp:id=3)]
            url: String,
            #[facet(capnp:id=4)]
            caption: String,
        },
        Video(#[facet(capnp:id=6)] String, #[facet(capnp:id=7)] u32),
    }

    #[test]
    fn test_enum_with_explicit_field_ids() {
        let union_struct = build_capnp_union_from_shape(EnumWithData::SHAPE).unwrap();
        let expected = Struct {
            name: "EnumWithData".to_string(),
            fields: vec![],
            union: Some(Union {
                variants: vec![
                    UnionVariant {
                        name: "myText".to_string(),
                        variant_inner: UnionVariantInner::Group(vec![CapnpField {
                            name: "field1".to_string(),
                            id: 1,
                            field_type: CapnpType::Text,
                        }]),
                    },
                    UnionVariant {
                        name: "image".to_string(),
                        variant_inner: UnionVariantInner::Group(vec![
                            CapnpField {
                                name: "url".to_string(),
                                id: 3,
                                field_type: CapnpType::Text,
                            },
                            CapnpField {
                                name: "caption".to_string(),
                                id: 4,
                                field_type: CapnpType::Text,
                            },
                        ]),
                    },
                    UnionVariant {
                        name: "video".to_string(),
                        variant_inner: UnionVariantInner::Group(vec![
                            CapnpField {
                                name: "field6".to_string(),
                                id: 6,
                                field_type: CapnpType::Text,
                            },
                            CapnpField {
                                name: "field7".to_string(),
                                id: 7,
                                field_type: CapnpType::UInt32,
                            },
                        ]),
                    },
                ],
            }),
            extra_fields: vec![],
        };
        assert_eq!(union_struct, expected);
    }

    // Test struct with extra fields for backwards compatibility
    #[derive(Facet)]
    #[facet(capnp:extra="deletedField @5 :UInt32")]
    #[facet(capnp:extra="anotherDeletedField @10 :Text")]
    struct StructWithExtraFields {
        #[facet(capnp:id=0)]
        active_field: String,
    }

    #[test]
    fn test_struct_with_extra_fields() {
        let capnp_struct = build_capnp_struct_from_shape(StructWithExtraFields::SHAPE).unwrap();

        let expected = Struct {
            name: "StructWithExtraFields".to_string(),
            fields: vec![CapnpField {
                name: "activeField".to_string(),
                id: 0,
                field_type: CapnpType::Text,
            }],
            union: None,
            extra_fields: vec![
                "deletedField @5 :UInt32".to_string(),
                "anotherDeletedField @10 :Text".to_string(),
            ],
        };

        assert_eq!(capnp_struct, expected);
    }

    // Test enum with extra fields for backwards compatibility
    #[derive(Facet)]
    #[facet(capnp:extra="deletedVariant @15 :Void")]
    #[repr(u8)]
    #[allow(dead_code)]
    enum EnumWithExtraFields {
        #[facet(capnp:id=0)]
        ActiveVariant,
    }

    #[test]
    fn test_enum_with_extra_fields() {
        let union_struct = build_capnp_union_from_shape(EnumWithExtraFields::SHAPE).unwrap();

        let expected = Struct {
            name: "EnumWithExtraFields".to_string(),
            fields: vec![],
            union: Some(Union {
                variants: vec![UnionVariant {
                    name: "activeVariant".to_string(),
                    variant_inner: UnionVariantInner::Type {
                        id: 0,
                        capnp_type: CapnpType::Void,
                    },
                }],
            }),
            extra_fields: vec!["deletedVariant @15 :Void".to_string()],
        };

        assert_eq!(union_struct, expected);
    }

    #[test]
    fn test_extra_fields_schema_rendering() {
        let schema = capnp_schema_for::<StructWithExtraFields>().unwrap();

        // Check that the extra fields are included in the rendered schema
        assert!(schema.contains("deletedField @5 :UInt32;"));
        assert!(schema.contains("anotherDeletedField @10 :Text;"));
        assert!(schema.contains("activeField @0 :Text;"));
    }

    // More comprehensive example showing extra fields in action
    #[derive(Facet)]
    #[facet(capnp:extra="oldUserId @1 :UInt64")]
    #[facet(capnp:extra="deprecatedFlag @3 :Bool")]
    #[facet(capnp:extra="removedTimestamp @7 :UInt64")]
    struct UserProfile {
        #[facet(capnp:id=0)]
        username: String,
        #[facet(capnp:id=2)]
        email: String,
        #[facet(capnp:id=4)]
        active: bool,
        #[facet(capnp:id=5)]
        tags: Vec<String>,
    }

    #[test]
    fn test_comprehensive_extra_fields_example() {
        let schema = capnp_schema_for::<UserProfile>().unwrap();

        println!("Generated Cap'n Proto schema:\n{}", schema);

        // Verify the schema contains both active fields and extra fields
        let expected_schema = "struct UserProfile {\n  username @0 :Text;\n  email @2 :Text;\n  active @4 :Bool;\n  tags @5 :List(Text);\n  oldUserId @1 :UInt64;\n  deprecatedFlag @3 :Bool;\n  removedTimestamp @7 :UInt64;\n}\n";
        assert_eq!(schema, expected_schema);
    }

    #[derive(Facet)]
    #[facet(capnp:extra="oldVariant @8 :Void")]
    #[facet(capnp:extra="deprecatedData @9 :UInt32")]
    #[repr(u8)]
    #[allow(dead_code)]
    enum MessageType {
        #[facet(capnp:id=0)]
        Text,
        Data {
            #[facet(capnp:id=1)]
            payload: Vec<u8>,
            #[facet(capnp:id=2)]
            format: String,
        },
    }

    #[test]
    fn test_enum_extra_fields_comprehensive() {
        let schema = capnp_schema_for::<MessageType>().unwrap();

        println!("Generated enum schema with extra fields:\n{}", schema);

        // Check that both union variants and extra fields are present
        assert!(schema.contains("text @0 :Void;"));
        assert!(schema.contains("data :group"));
        assert!(schema.contains("payload @1 :List(UInt8);"));
        assert!(schema.contains("format @2 :Text;"));
        assert!(schema.contains("oldVariant @8 :Void;"));
        assert!(schema.contains("deprecatedData @9 :UInt32;"));
    }

    #[test]
    fn test_build_capnp_file() {
        let shapes = &[TestStruct::SHAPE, Status::SHAPE];
        let file_id = 0xabcd1234u64;

        let result = build_capnp_file(file_id, shapes).unwrap();

        // Check that the file starts with the correct ID in hex
        assert!(result.starts_with("@0xabcd1234;"));

        // Check that both structs are present
        assert!(result.contains("struct TestStruct {"));
        assert!(result.contains("struct Status {"));

        // Check specific field content
        assert!(result.contains("fullName @1 :Text;"));
        assert!(result.contains("active @0 :Void;"));

        println!("Generated file:\n{}", result);
    }

    #[test]
    fn test_build_capnp_file_empty_shapes() {
        let shapes: &[&'static Shape] = &[];
        let file_id = 0x42u64;

        let result = build_capnp_file(file_id, shapes).unwrap();

        // Should just contain the file ID
        assert_eq!(result, "@0x42;\n\n");
    }
}
