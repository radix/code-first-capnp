use std::fmt::Write;

use facet::{
    Facet, Field, FieldAttribute, NumericType, PrimitiveType, SequenceType, Shape, ShapeLayout,
    StructKind, TextualType, Type, UserType,
};

/// Convention:
/// - Put `#[facet(capnp:id=<N>)]` on fields to specify field number (required)
/// - Optionally `#[facet(name=<foo>)]` to rename in the .capnp
pub fn capnp_struct_for<T: Facet<'static>>() -> Result<String, String> {
    let shape = T::SHAPE;

    let (st_name, st) = match shape.ty {
        Type::User(UserType::Struct(sd)) => (shape.type_identifier, sd),
        _ => return Err(format!("{} is not a struct", shape.type_identifier)),
    };

    // Only record/tuple structs are supported here; unit struct becomes empty record.
    if matches!(st.kind, StructKind::Unit) {
        return Ok(format!("struct {} {{}}\n", st_name));
    }

    // Build a list of (capnp_field_id, capnp_field_name, capnp_type_token)
    let mut fields_out = Vec::<(u32, String, String)>::new();

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

        // Map facet type → Cap'n Proto type token
        let capnp_ty = map_capnp_type(fld.shape)?;

        fields_out.push((id, capnp_name, capnp_ty));
    }

    // Cap'n Proto schemas don't require sorted ids, but it's conventional.
    fields_out.sort_by_key(|(id, _, _)| *id);

    // Render
    let mut out = String::new();
    writeln!(&mut out, "struct {} {{", st_name).unwrap();
    for (id, name, ty) in fields_out {
        // e.g., "  id @0 :UInt64;"
        writeln!(&mut out, "  {} @{} :{};", name, id, ty).unwrap();
    }
    writeln!(&mut out, "}}").unwrap();

    Ok(out)
}

/// Generate a Cap'n Proto union for a Rust enum
pub fn capnp_union_for<T: Facet<'static>>() -> Result<String, String> {
    let shape = T::SHAPE;

    let (enum_name, enum_def) = match shape.ty {
        Type::User(UserType::Enum(ed)) => (shape.type_identifier, ed),
        _ => return Err(format!("{} is not an enum", shape.type_identifier)),
    };

    // Build union variants
    let mut variants_out = Vec::<(u32, String, String)>::new();

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

        let capnp_type = match variant.data.kind {
            StructKind::Unit => {
                // Unit variants become Void in Cap'n Proto
                "Void".to_string()
            }
            StructKind::Tuple => {
                // Tuple variants need their own struct definition
                if variant.data.fields.is_empty() {
                    "Void".to_string()
                } else {
                    // Create a reference to a separate struct for tuple data
                    format!("{}_{}", enum_name, variant_name)
                }
            }
            StructKind::TupleStruct => {
                // TupleStruct variants need their own struct definition
                if variant.data.fields.is_empty() {
                    "Void".to_string()
                } else {
                    // Create a reference to a separate struct for tuple struct data
                    format!("{}_{}", enum_name, variant_name)
                }
            }
            StructKind::Struct => {
                // Named struct variants need their own struct definition
                // For now, we'll create a reference to a separate struct
                format!("{}_{}", enum_name, variant_name)
            }
        };

        variants_out.push((variant_id, variant_name.to_string(), capnp_type));
    }

    // Render union
    let mut out = String::new();
    writeln!(&mut out, "struct {} {{", enum_name).unwrap();
    writeln!(&mut out, "  union {{").unwrap();
    for (id, name, ty) in variants_out {
        writeln!(&mut out, "    {} @{} :{};", name, id, ty).unwrap();
    }
    writeln!(&mut out, "  }}").unwrap();
    writeln!(&mut out, "}}").unwrap();

    Ok(out)
}

/// Generate helper structs for enum variants that have associated data
pub fn capnp_enum_variant_structs_for<T: Facet<'static>>() -> Result<String, String> {
    let shape = T::SHAPE;

    let (enum_name, enum_def) = match shape.ty {
        Type::User(UserType::Enum(ed)) => (shape.type_identifier, ed),
        _ => return Err(format!("{} is not an enum", shape.type_identifier)),
    };

    let mut out = String::new();

    for variant in enum_def.variants.iter() {
        let variant_name = variant.name;

        // Only generate structs for variants that have associated data
        match variant.data.kind {
            StructKind::Unit => {
                // Unit variants don't need helper structs
                continue;
            }
            StructKind::Tuple | StructKind::TupleStruct => {
                if variant.data.fields.is_empty() {
                    continue;
                }

                // Generate struct for tuple/tuple-struct variant
                writeln!(&mut out, "struct {}_{} {{", enum_name, variant_name).unwrap();
                for (field_idx, field) in variant.data.fields.iter().enumerate() {
                    let field_name = format!("field{}", field_idx);
                    let capnp_ty = map_capnp_type(field.shape)?;
                    writeln!(&mut out, "  {} @{} :{};", field_name, field_idx, capnp_ty).unwrap();
                }
                writeln!(&mut out, "}}").unwrap();
                writeln!(&mut out).unwrap();
            }
            StructKind::Struct => {
                // Generate struct for named struct variant
                writeln!(&mut out, "struct {}_{} {{", enum_name, variant_name).unwrap();
                for (field_idx, field) in variant.data.fields.iter().enumerate() {
                    let field_name = field.name;
                    let capnp_ty = map_capnp_type(field.shape)?;
                    writeln!(&mut out, "  {} @{} :{};", field_name, field_idx, capnp_ty).unwrap();
                }
                writeln!(&mut out, "}}").unwrap();
                writeln!(&mut out).unwrap();
            }
        }
    }

    Ok(out)
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
fn map_capnp_type(shape: &'static Shape) -> Result<String, String> {
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
                        (1, false) => "UInt8",
                        (2, false) => "UInt16",
                        (4, false) => "UInt32",
                        (8, false) => "UInt64",
                        (16, false) => return Err("UInt128 not supported in Cap'n Proto".into()),
                        (1, true) => "Int8",
                        (2, true) => "Int16",
                        (4, true) => "Int32",
                        (8, true) => "Int64",
                        (16, true) => return Err("Int128 not supported in Cap'n Proto".into()),
                        _ => return Err(format!("Unsupported integer size: {} bytes", size_bytes)),
                    },
                    NumericType::Float => match size_bytes {
                        4 => "Float32",
                        8 => "Float64",
                        _ => return Err(format!("Unsupported float size: {} bytes", size_bytes)),
                    },
                }
                .into()
            }
            PrimitiveType::Boolean => "Bool".into(),
            PrimitiveType::Textual(t) => match t {
                TextualType::Str | TextualType::Char => "Text".into(), // store char as 1-char Text for now
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

            let inner_capnp_type = map_capnp_type(inner_shape)?;
            Ok(format!("List({})", inner_capnp_type))
        }

        Type::User(user_type) => {
            match user_type {
                UserType::Struct(_) => {
                    // Reference by type name — assume you'll emit that struct separately
                    Ok(shape.type_identifier.to_string())
                }
                UserType::Enum(_) => {
                    // Enums become unions in Cap'n Proto - reference by type name
                    Ok(shape.type_identifier.to_string())
                }
                UserType::Opaque => {
                    // Handle common opaque types based on their type identifier
                    match shape.type_identifier {
                        "String" => Ok("Text".into()),
                        "Vec" => {
                            // For Vec<T>, we need to look at the type parameters to get T
                            if let Some(type_param) = shape.type_params.first() {
                                let inner_capnp_type = map_capnp_type((type_param.shape)())?;
                                Ok(format!("List({})", inner_capnp_type))
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
    fn test_basic_struct_generation() {
        let result = capnp_struct_for::<TestStruct>().unwrap();
        println!("{}", result);

        // Should contain the struct definition
        assert!(result.contains("struct TestStruct {"));
        assert!(result.contains("id @0 :UInt64;"));
        assert!(result.contains("fullName @1 :Text;"));
        assert!(result.contains("numbers @2 :List(Int32);"));
        assert!(result.contains("active @3 :Bool;"));
    }

    #[derive(Facet)]
    struct EmptyStruct;

    #[test]
    fn test_unit_struct() {
        let result = capnp_struct_for::<EmptyStruct>().unwrap();
        println!("{}", result);
        assert!(result.contains("struct EmptyStruct {}"));
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
        let result = capnp_struct_for::<MissingIdStruct>();
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
    fn test_enum_union_generation() {
        let result = capnp_union_for::<Status>().unwrap();
        println!("{}", result);

        // Should contain the union definition
        assert!(result.contains("struct Status {"));
        assert!(result.contains("union {"));
        assert!(result.contains("Active @0 :Void;"));
        assert!(result.contains("Inactive @1 :Void;"));
        assert!(result.contains("Pending @2 :Void;"));
        assert!(result.contains("}"));
    }

    #[test]
    fn test_complex_enum_union_generation() {
        let result = capnp_union_for::<ComplexEnum>().unwrap();
        println!("{}", result);

        // Should contain the union definition with different variant types
        assert!(result.contains("struct ComplexEnum {"));
        assert!(result.contains("union {"));
        assert!(result.contains("Unit @0 :Void;"));
        assert!(result.contains("Tuple @1 :ComplexEnum_Tuple;"));
        assert!(result.contains("Struct @2 :ComplexEnum_Struct;"));
        assert!(result.contains("}"));
    }

    #[test]
    fn test_enum_variant_structs_generation() {
        let result = capnp_enum_variant_structs_for::<ComplexEnum>().unwrap();
        println!("{}", result);

        // Should generate helper structs for variants with data
        assert!(result.contains("struct ComplexEnum_Tuple {"));
        assert!(result.contains("field0 @0 :UInt32;"));
        assert!(result.contains("field1 @1 :Text;"));

        assert!(result.contains("struct ComplexEnum_Struct {"));
        assert!(result.contains("id @0 :UInt64;"));
        assert!(result.contains("name @1 :Text;"));

        // Should not generate struct for Unit variant
        assert!(!result.contains("ComplexEnum_Unit"));
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
        let result = capnp_union_for::<MissingIdEnum>();
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("Variant 'MissingId' missing required capnp:id attribute"));
        assert!(error_msg.contains("#[facet(capnp:id=N)]"));
    }
}
