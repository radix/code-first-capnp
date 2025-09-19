use std::fmt::Write;

use facet::{
    Facet, Shape, Type, UserType,
    PrimitiveType, TextualType, SequenceType, StructKind,
    Field, FieldAttribute, NumericType, ShapeLayout,
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
                    ShapeLayout::Unsized => return Err("Cannot handle unsized numeric types".into()),
                };
                let size_bytes = layout.size();

                match n {
                    NumericType::Integer { signed } => {
                        match (size_bytes, signed) {
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
                        }
                    }
                    NumericType::Float => {
                        match size_bytes {
                            4 => "Float32",
                            8 => "Float64",
                            _ => return Err(format!("Unsupported float size: {} bytes", size_bytes)),
                        }
                    }
                }.into()
            },
            PrimitiveType::Boolean => "Bool".into(),
            PrimitiveType::Textual(t) => match t {
                TextualType::Str | TextualType::Char => "Text".into(), // store char as 1-char Text for now
            },
            PrimitiveType::Never => return Err("Never type (!) cannot be represented in Cap'n Proto".into()),
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
                    // Basic enums can be modeled as an `enum` (not shown here). If you store them in a struct
                    // field, point to a type with a separate enum definition.
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
                        _ => Err(format!("Unsupported opaque type: {}", shape.type_identifier)),
                    }
                }
                UserType::Union(_) => Err("Union types not yet supported".into()),
            }
        }

        Type::Pointer(_) => Err("pointers/smart-pointers not directly supported in Cap'n Proto; wrap/flatten".into()),
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
}
