//! Abstract document model for Cap'n Proto schemas.
//!
//! This module defines data structures that represent Cap'n Proto schemas
//! in an abstract way, separate from the string generation logic.

use std::fmt::Write;

/// Error type for Cap'n Proto model validation
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    DuplicateId { id: u32, locations: Vec<String> },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::DuplicateId { id, locations } => {
                write!(f, "Duplicate ID {} found in: {}", id, locations.join(", "))
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Represents a complete Cap'n Proto schema document
#[derive(Debug, Clone, PartialEq)]
pub struct CapnpDocument {
    pub items: Vec<CapnpItem>,
}

/// Top-level items in a Cap'n Proto schema
#[derive(Debug, Clone, PartialEq)]
pub enum CapnpItem {
    Struct(CapnpStruct),
}

/// Represents a Cap'n Proto struct definition
#[derive(Debug, Clone, PartialEq)]
pub struct CapnpStruct {
    pub name: String,
    pub fields: Vec<CapnpField>,
    pub union: Option<CapnpUnion>,
}

/// Represents a field in a Cap'n Proto struct
#[derive(Debug, Clone, PartialEq)]
pub struct CapnpField {
    pub name: String,
    pub id: u32,
    pub field_type: CapnpType,
}

/// Represents a union within a Cap'n Proto struct
#[derive(Debug, Clone, PartialEq)]
pub struct CapnpUnion {
    pub variants: Vec<CapnpUnionVariant>,
}

/// Represents a variant within a Cap'n Proto union
#[derive(Debug, Clone, PartialEq)]
pub struct CapnpUnionVariant {
    pub name: String,
    pub id: u32,
    pub variant_type: CapnpVariantType,
}

/// Represents the type of a union variant (either a type or a group)
#[derive(Debug, Clone, PartialEq)]
pub enum CapnpVariantType {
    Type(CapnpType),
    Group(Vec<CapnpField>),
}

/// Represents Cap'n Proto types
#[derive(Debug, Clone, PartialEq)]
pub enum CapnpType {
    // Primitive types
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float32,
    Float64,
    Text,
    Void,

    // Complex types
    List(Box<CapnpType>),

    // User-defined types (referenced by name)
    UserDefined(String),
}

impl CapnpDocument {
    /// Creates a new empty document
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Adds an item to the document
    pub fn add_item(&mut self, item: CapnpItem) {
        self.items.push(item);
    }

    /// Creates a document with a single struct
    pub fn with_struct(capnp_struct: CapnpStruct) -> Self {
        Self {
            items: vec![CapnpItem::Struct(capnp_struct)],
        }
    }

    /// Validates all structs in the document for ID conflicts
    pub fn validate(&self) -> Result<(), ValidationError> {
        for item in &self.items {
            match item {
                CapnpItem::Struct(s) => s.validate()?,
            }
        }
        Ok(())
    }

    /// Renders the document as Cap'n Proto schema text
    /// Automatically validates all structs before rendering
    pub fn render(&self) -> Result<String, ValidationError> {
        // Validate before rendering
        self.validate()?;

        let mut output = String::new();

        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                writeln!(&mut output).unwrap();
            }
            write!(&mut output, "{}", item.render()?).unwrap();
        }

        Ok(output)
    }
}

impl CapnpItem {
    /// Renders the item as Cap'n Proto schema text
    pub fn render(&self) -> Result<String, ValidationError> {
        match self {
            CapnpItem::Struct(s) => s.render(),
        }
    }
}

impl CapnpStruct {
    /// Creates a new struct with the given name
    pub fn new(name: String) -> Self {
        Self {
            name,
            fields: Vec::new(),
            union: None,
        }
    }

    /// Adds a field to the struct
    pub fn add_field(&mut self, field: CapnpField) {
        self.fields.push(field);
    }

    /// Sets the union for this struct
    pub fn set_union(&mut self, union: CapnpUnion) {
        self.union = Some(union);
    }

    /// Validates that all IDs in the struct are unique
    /// This includes regular field IDs, union variant IDs, and union group field IDs
    pub fn validate(&self) -> Result<(), ValidationError> {
        let mut id_locations: std::collections::HashMap<u32, Vec<String>> =
            std::collections::HashMap::new();

        // Collect regular field IDs
        for field in &self.fields {
            let location = format!("struct field '{}'", field.name);
            id_locations.entry(field.id).or_default().push(location);
        }

        // Collect union variant and group field IDs if union exists
        if let Some(union) = &self.union {
            for variant in &union.variants {
                let location = format!("union variant '{}'", variant.name);
                id_locations.entry(variant.id).or_default().push(location);

                // If this variant is a group, collect its field IDs too
                if let CapnpVariantType::Group(fields) = &variant.variant_type {
                    for field in fields {
                        let location =
                            format!("union group '{}' field '{}'", variant.name, field.name);
                        id_locations.entry(field.id).or_default().push(location);
                    }
                }
            }
        }

        // Check for duplicates
        for (id, locations) in id_locations {
            if locations.len() > 1 {
                return Err(ValidationError::DuplicateId { id, locations });
            }
        }

        Ok(())
    }

    /// Renders the struct as Cap'n Proto schema text
    /// Automatically validates the struct before rendering
    pub fn render(&self) -> Result<String, ValidationError> {
        // Validate before rendering
        self.validate()?;

        let mut output = String::new();

        writeln!(&mut output, "struct {} {{", self.name).unwrap();

        // Render regular fields
        for field in &self.fields {
            writeln!(&mut output, "  {}", field.render()).unwrap();
        }

        // Render union if present
        if let Some(union) = &self.union {
            write!(&mut output, "{}", union.render()).unwrap();
        }

        writeln!(&mut output, "}}").unwrap();

        Ok(output)
    }
}

impl CapnpField {
    /// Creates a new field
    pub fn new(name: String, id: u32, field_type: CapnpType) -> Self {
        Self {
            name,
            id,
            field_type,
        }
    }

    /// Renders the field as Cap'n Proto schema text
    pub fn render(&self) -> String {
        format!("{} @{} :{};", self.name, self.id, self.field_type.render())
    }
}

impl CapnpUnion {
    /// Creates a new union
    pub fn new() -> Self {
        Self {
            variants: Vec::new(),
        }
    }

    /// Adds a variant to the union
    pub fn add_variant(&mut self, variant: CapnpUnionVariant) {
        self.variants.push(variant);
    }

    /// Renders the union as Cap'n Proto schema text
    pub fn render(&self) -> String {
        let mut output = String::new();

        writeln!(&mut output, "  union {{").unwrap();
        for variant in &self.variants {
            writeln!(&mut output, "    {}", variant.render()).unwrap();
        }
        writeln!(&mut output, "  }}").unwrap();

        output
    }
}

impl CapnpUnionVariant {
    /// Creates a new union variant with a type
    pub fn new(name: String, id: u32, variant_type: CapnpType) -> Self {
        Self {
            name,
            id,
            variant_type: CapnpVariantType::Type(variant_type),
        }
    }

    /// Creates a new union variant with a group
    pub fn new_group(name: String, id: u32, fields: Vec<CapnpField>) -> Self {
        Self {
            name,
            id,
            variant_type: CapnpVariantType::Group(fields),
        }
    }

    /// Renders the variant as Cap'n Proto schema text
    pub fn render(&self) -> String {
        match &self.variant_type {
            CapnpVariantType::Type(ty) => {
                format!("{} @{} :{};", self.name, self.id, ty.render())
            }
            CapnpVariantType::Group(fields) => {
                if fields.is_empty() {
                    format!("{} :group @{} {{}};", self.name, self.id)
                } else {
                    let mut output = String::new();
                    output.push_str(&format!("{} :group @{} {{\n", self.name, self.id));
                    for field in fields {
                        output.push_str(&format!("      {}\n", field.render()));
                    }
                    output.push_str("    };");
                    output
                }
            }
        }
    }
}

impl CapnpType {
    /// Renders the type as Cap'n Proto schema text
    pub fn render(&self) -> String {
        match self {
            CapnpType::Bool => "Bool".to_string(),
            CapnpType::Int8 => "Int8".to_string(),
            CapnpType::Int16 => "Int16".to_string(),
            CapnpType::Int32 => "Int32".to_string(),
            CapnpType::Int64 => "Int64".to_string(),
            CapnpType::UInt8 => "UInt8".to_string(),
            CapnpType::UInt16 => "UInt16".to_string(),
            CapnpType::UInt32 => "UInt32".to_string(),
            CapnpType::UInt64 => "UInt64".to_string(),
            CapnpType::Float32 => "Float32".to_string(),
            CapnpType::Float64 => "Float64".to_string(),
            CapnpType::Text => "Text".to_string(),
            CapnpType::Void => "Void".to_string(),
            CapnpType::List(inner) => format!("List({})", inner.render()),
            CapnpType::UserDefined(name) => name.clone(),
        }
    }
}

impl Default for CapnpDocument {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for CapnpUnion {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Document tests
    #[test]
    fn test_empty_document() {
        let doc = CapnpDocument::new();
        assert_eq!(doc.items.len(), 0);
        assert_eq!(doc.render().unwrap(), "");
    }

    #[test]
    fn test_document_default() {
        let doc = CapnpDocument::default();
        assert_eq!(doc.items.len(), 0);
    }

    #[test]
    fn test_document_with_struct() {
        let s = CapnpStruct::new("Test".to_string());
        let doc = CapnpDocument::with_struct(s);

        assert_eq!(doc.items.len(), 1);
        assert!(matches!(doc.items[0], CapnpItem::Struct(_)));
    }

    #[test]
    fn test_document_add_item() {
        let mut doc = CapnpDocument::new();
        let s = CapnpStruct::new("Test".to_string());

        doc.add_item(CapnpItem::Struct(s));
        assert_eq!(doc.items.len(), 1);
    }

    #[test]
    fn test_multiple_structs_with_spacing() {
        let mut doc = CapnpDocument::new();

        let s1 = CapnpStruct::new("Person".to_string());
        let s2 = CapnpStruct::new("Company".to_string());

        doc.add_item(CapnpItem::Struct(s1));
        doc.add_item(CapnpItem::Struct(s2));

        let output = doc.render().unwrap();

        assert!(output.contains("struct Person {"));
        assert!(output.contains("struct Company {"));
        // Test that there's spacing between structs
        assert!(output.contains("}\n\nstruct Company"));
    }

    // Struct tests
    #[test]
    fn test_empty_struct() {
        let s = CapnpStruct::new("Empty".to_string());
        let output = s.render().unwrap();

        assert_eq!(output, "struct Empty {\n}\n");
    }

    #[test]
    fn test_struct_new() {
        let s = CapnpStruct::new("TestStruct".to_string());

        assert_eq!(s.name, "TestStruct");
        assert_eq!(s.fields.len(), 0);
        assert!(s.union.is_none());
    }

    #[test]
    fn test_struct_add_field() {
        let mut s = CapnpStruct::new("Test".to_string());
        let field = CapnpField::new("test".to_string(), 0, CapnpType::Bool);

        s.add_field(field);
        assert_eq!(s.fields.len(), 1);
        assert_eq!(s.fields[0].name, "test");
    }

    #[test]
    fn test_struct_set_union() {
        let mut s = CapnpStruct::new("Test".to_string());
        let union = CapnpUnion::new();

        s.set_union(union);
        assert!(s.union.is_some());
    }

    #[test]
    fn test_struct_with_fields_and_union() {
        let mut s = CapnpStruct::new("Complex".to_string());

        s.add_field(CapnpField::new("id".to_string(), 0, CapnpType::UInt64));

        let mut union = CapnpUnion::new();
        union.add_variant(CapnpUnionVariant::new(
            "variant".to_string(),
            1,
            CapnpType::Void,
        ));
        s.set_union(union);

        let output = s.render().unwrap();

        assert!(output.contains("id @0 :UInt64;"));
        assert!(output.contains("union {"));
        assert!(output.contains("variant @1 :Void;"));
    }

    // Field tests
    #[test]
    fn test_field_new() {
        let field = CapnpField::new("test".to_string(), 5, CapnpType::Text);

        assert_eq!(field.name, "test");
        assert_eq!(field.id, 5);
        assert_eq!(field.field_type, CapnpType::Text);
    }

    #[test]
    fn test_field_render() {
        let field = CapnpField::new("myField".to_string(), 42, CapnpType::Float32);
        let output = field.render();

        assert_eq!(output, "myField @42 :Float32;");
    }

    // Union tests
    #[test]
    fn test_union_new() {
        let union = CapnpUnion::new();
        assert_eq!(union.variants.len(), 0);
    }

    #[test]
    fn test_union_default() {
        let union = CapnpUnion::default();
        assert_eq!(union.variants.len(), 0);
    }

    #[test]
    fn test_union_add_variant() {
        let mut union = CapnpUnion::new();
        let variant = CapnpUnionVariant::new("test".to_string(), 0, CapnpType::Void);

        union.add_variant(variant);
        assert_eq!(union.variants.len(), 1);
    }

    #[test]
    fn test_union_add_group_variant() {
        let mut union = CapnpUnion::new();
        let fields = vec![
            CapnpField::new("field1".to_string(), 0, CapnpType::UInt32),
            CapnpField::new("field2".to_string(), 1, CapnpType::Text),
        ];
        let variant = CapnpUnionVariant::new_group("test".to_string(), 0, fields);

        union.add_variant(variant);
        assert_eq!(union.variants.len(), 1);
    }

    #[test]
    fn test_empty_union_render() {
        let union = CapnpUnion::new();
        let output = union.render();

        assert_eq!(output, "  union {\n  }\n");
    }

    // Union variant tests
    #[test]
    fn test_union_variant_render() {
        let variant = CapnpUnionVariant::new("myVariant".to_string(), 10, CapnpType::Text);
        let output = variant.render();

        assert_eq!(output, "myVariant @10 :Text;");
    }

    #[test]
    fn test_union_variant_empty_group_render() {
        let variant = CapnpUnionVariant::new_group("emptyGroup".to_string(), 5, vec![]);
        let output = variant.render();

        assert_eq!(output, "emptyGroup :group @5 {};");
    }

    #[test]
    fn test_union_variant_group_render() {
        let fields = vec![
            CapnpField::new("id".to_string(), 0, CapnpType::UInt64),
            CapnpField::new("name".to_string(), 1, CapnpType::Text),
        ];
        let variant = CapnpUnionVariant::new_group("myGroup".to_string(), 2, fields);
        let output = variant.render();

        let expected = "myGroup :group @2 {\n      id @0 :UInt64;\n      name @1 :Text;\n    };";
        assert_eq!(output, expected);
    }

    // CapnpType tests - primitive types
    #[test]
    fn test_all_primitive_types() {
        assert_eq!(CapnpType::Bool.render(), "Bool");
        assert_eq!(CapnpType::Int8.render(), "Int8");
        assert_eq!(CapnpType::Int16.render(), "Int16");
        assert_eq!(CapnpType::Int32.render(), "Int32");
        assert_eq!(CapnpType::Int64.render(), "Int64");
        assert_eq!(CapnpType::UInt8.render(), "UInt8");
        assert_eq!(CapnpType::UInt16.render(), "UInt16");
        assert_eq!(CapnpType::UInt32.render(), "UInt32");
        assert_eq!(CapnpType::UInt64.render(), "UInt64");
        assert_eq!(CapnpType::Float32.render(), "Float32");
        assert_eq!(CapnpType::Float64.render(), "Float64");
        assert_eq!(CapnpType::Text.render(), "Text");
        assert_eq!(CapnpType::Void.render(), "Void");
    }

    #[test]
    fn test_user_defined_type() {
        let user_type = CapnpType::UserDefined("MyCustomType".to_string());
        assert_eq!(user_type.render(), "MyCustomType");
    }

    #[test]
    fn test_simple_list_types() {
        let list_bool = CapnpType::List(Box::new(CapnpType::Bool));
        assert_eq!(list_bool.render(), "List(Bool)");

        let list_text = CapnpType::List(Box::new(CapnpType::Text));
        assert_eq!(list_text.render(), "List(Text)");

        let list_user = CapnpType::List(Box::new(CapnpType::UserDefined("Custom".to_string())));
        assert_eq!(list_user.render(), "List(Custom)");
    }

    #[test]
    fn test_nested_list_types() {
        let nested_list = CapnpType::List(Box::new(CapnpType::List(Box::new(CapnpType::UInt32))));
        assert_eq!(nested_list.render(), "List(List(UInt32))");

        let deeply_nested = CapnpType::List(Box::new(CapnpType::List(Box::new(CapnpType::List(
            Box::new(CapnpType::Bool),
        )))));
        assert_eq!(deeply_nested.render(), "List(List(List(Bool)))");
    }

    // Integration tests
    #[test]
    fn test_simple_struct_rendering() {
        let mut s = CapnpStruct::new("Person".to_string());
        s.add_field(CapnpField::new("id".to_string(), 0, CapnpType::UInt64));
        s.add_field(CapnpField::new("name".to_string(), 1, CapnpType::Text));

        let doc = CapnpDocument::with_struct(s);
        let output = doc.render().unwrap();

        assert!(output.contains("struct Person {"));
        assert!(output.contains("id @0 :UInt64;"));
        assert!(output.contains("name @1 :Text;"));
        assert!(output.contains("}"));
    }

    #[test]
    fn test_union_struct_rendering() {
        let mut s = CapnpStruct::new("Message".to_string());

        let mut union = CapnpUnion::new();
        union.add_variant(CapnpUnionVariant::new(
            "text".to_string(),
            0,
            CapnpType::Text,
        ));
        union.add_variant(CapnpUnionVariant::new(
            "number".to_string(),
            1,
            CapnpType::UInt32,
        ));

        s.set_union(union);

        let doc = CapnpDocument::with_struct(s);
        let output = doc.render().unwrap();

        assert!(output.contains("struct Message {"));
        assert!(output.contains("union {"));
        assert!(output.contains("text @0 :Text;"));
        assert!(output.contains("number @1 :UInt32;"));
        assert!(output.contains("  }"));
        assert!(output.contains("}"));
    }

    #[test]
    fn test_union_struct_with_groups_rendering() {
        let mut s = CapnpStruct::new("ComplexMessage".to_string());

        let mut union = CapnpUnion::new();
        union.add_variant(CapnpUnionVariant::new(
            "unit".to_string(),
            0,
            CapnpType::Void,
        ));

        let tuple_fields = vec![
            CapnpField::new("field0".to_string(), 1, CapnpType::UInt32),
            CapnpField::new("field1".to_string(), 2, CapnpType::Text),
        ];
        union.add_variant(CapnpUnionVariant::new_group(
            "tuple".to_string(),
            3,
            tuple_fields,
        ));

        let struct_fields = vec![
            CapnpField::new("id".to_string(), 4, CapnpType::UInt64),
            CapnpField::new("name".to_string(), 5, CapnpType::Text),
        ];
        union.add_variant(CapnpUnionVariant::new_group(
            "struct".to_string(),
            6,
            struct_fields,
        ));

        s.set_union(union);

        let doc = CapnpDocument::with_struct(s);
        let output = doc.render().unwrap();

        assert!(output.contains("struct ComplexMessage {"));
        assert!(output.contains("union {"));
        assert!(output.contains("unit @0 :Void;"));
        assert!(output.contains("tuple :group @3 {"));
        assert!(output.contains("field0 @1 :UInt32;"));
        assert!(output.contains("field1 @2 :Text;"));
        assert!(output.contains("struct :group @6 {"));
        assert!(output.contains("id @4 :UInt64;"));
        assert!(output.contains("name @5 :Text;"));
    }

    #[test]
    fn test_complex_struct_with_all_types() {
        let mut s = CapnpStruct::new("ComplexStruct".to_string());

        // Add fields with various types
        s.add_field(CapnpField::new("boolField".to_string(), 0, CapnpType::Bool));
        s.add_field(CapnpField::new("intField".to_string(), 1, CapnpType::Int32));
        s.add_field(CapnpField::new(
            "floatField".to_string(),
            2,
            CapnpType::Float64,
        ));
        s.add_field(CapnpField::new("textField".to_string(), 3, CapnpType::Text));
        s.add_field(CapnpField::new(
            "listField".to_string(),
            4,
            CapnpType::List(Box::new(CapnpType::UInt32)),
        ));
        s.add_field(CapnpField::new(
            "customField".to_string(),
            5,
            CapnpType::UserDefined("CustomType".to_string()),
        ));

        // Add union
        let mut union = CapnpUnion::new();
        union.add_variant(CapnpUnionVariant::new(
            "voidVariant".to_string(),
            6,
            CapnpType::Void,
        ));
        union.add_variant(CapnpUnionVariant::new(
            "textVariant".to_string(),
            7,
            CapnpType::Text,
        ));
        s.set_union(union);

        let output = s.render().unwrap();

        // Check all field types are rendered correctly
        assert!(output.contains("boolField @0 :Bool;"));
        assert!(output.contains("intField @1 :Int32;"));
        assert!(output.contains("floatField @2 :Float64;"));
        assert!(output.contains("textField @3 :Text;"));
        assert!(output.contains("listField @4 :List(UInt32);"));
        assert!(output.contains("customField @5 :CustomType;"));

        // Check union variants
        assert!(output.contains("voidVariant @6 :Void;"));
        assert!(output.contains("textVariant @7 :Text;"));
    }

    // Validation tests
    #[test]
    fn test_valid_struct_with_unique_ids() {
        let mut s = CapnpStruct::new("ValidStruct".to_string());
        s.add_field(CapnpField::new("field1".to_string(), 0, CapnpType::UInt32));
        s.add_field(CapnpField::new("field2".to_string(), 1, CapnpType::Text));

        let mut union = CapnpUnion::new();
        union.add_variant(CapnpUnionVariant::new(
            "variant1".to_string(),
            2,
            CapnpType::Void,
        ));

        let group_fields = vec![
            CapnpField::new("groupField1".to_string(), 3, CapnpType::UInt64),
            CapnpField::new("groupField2".to_string(), 4, CapnpType::Text),
        ];
        union.add_variant(CapnpUnionVariant::new_group(
            "group1".to_string(),
            5,
            group_fields,
        ));
        s.set_union(union);

        assert!(s.validate().is_ok());
    }

    #[test]
    fn test_duplicate_field_ids_in_struct() {
        let mut s = CapnpStruct::new("InvalidStruct".to_string());
        s.add_field(CapnpField::new("field1".to_string(), 0, CapnpType::UInt32));
        s.add_field(CapnpField::new("field2".to_string(), 0, CapnpType::Text)); // Duplicate ID

        let result = s.validate();
        assert!(result.is_err());

        if let Err(ValidationError::DuplicateId { id, locations }) = result {
            assert_eq!(id, 0);
            assert_eq!(locations.len(), 2);
            assert!(locations.contains(&"struct field 'field1'".to_string()));
            assert!(locations.contains(&"struct field 'field2'".to_string()));
        } else {
            panic!("Expected DuplicateId error");
        }
    }

    #[test]
    fn test_duplicate_struct_field_and_union_variant_id() {
        let mut s = CapnpStruct::new("InvalidStruct".to_string());
        s.add_field(CapnpField::new("field1".to_string(), 0, CapnpType::UInt32));

        let mut union = CapnpUnion::new();
        union.add_variant(CapnpUnionVariant::new(
            "variant1".to_string(),
            0,
            CapnpType::Void,
        )); // Duplicate ID
        s.set_union(union);

        let result = s.validate();
        assert!(result.is_err());

        if let Err(ValidationError::DuplicateId { id, locations }) = result {
            assert_eq!(id, 0);
            assert_eq!(locations.len(), 2);
            assert!(locations.contains(&"struct field 'field1'".to_string()));
            assert!(locations.contains(&"union variant 'variant1'".to_string()));
        } else {
            panic!("Expected DuplicateId error");
        }
    }

    #[test]
    fn test_duplicate_group_field_ids() {
        let mut s = CapnpStruct::new("InvalidStruct".to_string());

        let mut union = CapnpUnion::new();

        let group_fields = vec![
            CapnpField::new("groupField1".to_string(), 0, CapnpType::UInt64),
            CapnpField::new("groupField2".to_string(), 0, CapnpType::Text), // Duplicate ID within group
        ];
        union.add_variant(CapnpUnionVariant::new_group(
            "group1".to_string(),
            1,
            group_fields,
        ));
        s.set_union(union);

        let result = s.validate();
        assert!(result.is_err());

        if let Err(ValidationError::DuplicateId { id, locations }) = result {
            assert_eq!(id, 0);
            assert_eq!(locations.len(), 2);
            assert!(locations.contains(&"union group 'group1' field 'groupField1'".to_string()));
            assert!(locations.contains(&"union group 'group1' field 'groupField2'".to_string()));
        } else {
            panic!("Expected DuplicateId error");
        }
    }

    #[test]
    fn test_multiple_duplicate_ids() {
        let mut s = CapnpStruct::new("InvalidStruct".to_string());
        s.add_field(CapnpField::new("field1".to_string(), 0, CapnpType::UInt32));
        s.add_field(CapnpField::new("field2".to_string(), 0, CapnpType::Text)); // Duplicate ID 0
        s.add_field(CapnpField::new("field3".to_string(), 1, CapnpType::Bool));

        let mut union = CapnpUnion::new();
        union.add_variant(CapnpUnionVariant::new(
            "variant1".to_string(),
            1,
            CapnpType::Void,
        )); // Duplicate ID 1
        s.set_union(union);

        let result = s.validate();
        assert!(result.is_err());

        // Should return error for the first duplicate found (order may vary due to HashMap)
        if let Err(ValidationError::DuplicateId { id, locations }) = result {
            assert!(id == 0 || id == 1);
            assert_eq!(locations.len(), 2);
        } else {
            panic!("Expected DuplicateId error");
        }
    }

    #[test]
    fn test_document_validation_success() {
        let mut doc = CapnpDocument::new();

        let mut s1 = CapnpStruct::new("Struct1".to_string());
        s1.add_field(CapnpField::new("field1".to_string(), 0, CapnpType::UInt32));
        doc.add_item(CapnpItem::Struct(s1));

        let mut s2 = CapnpStruct::new("Struct2".to_string());
        s2.add_field(CapnpField::new("field1".to_string(), 0, CapnpType::Text)); // Same ID in different struct is OK
        doc.add_item(CapnpItem::Struct(s2));

        assert!(doc.validate().is_ok());
    }

    #[test]
    fn test_duplicate_ids_between_different_union_groups() {
        let mut s = CapnpStruct::new("InvalidStruct".to_string());
        s.add_field(CapnpField::new(
            "regularField".to_string(),
            0,
            CapnpType::UInt32,
        ));

        let mut union = CapnpUnion::new();

        // First group with fields having IDs 1 and 2
        let group1_fields = vec![
            CapnpField::new("width".to_string(), 1, CapnpType::UInt32),
            CapnpField::new("height".to_string(), 2, CapnpType::UInt32),
        ];
        union.add_variant(CapnpUnionVariant::new_group(
            "dimensions".to_string(),
            3,
            group1_fields,
        ));

        // Second group with field having ID 1 (duplicate with first group)
        let group2_fields = vec![
            CapnpField::new("name".to_string(), 1, CapnpType::Text), // Duplicate ID with dimensions.width
            CapnpField::new("description".to_string(), 4, CapnpType::Text),
        ];
        union.add_variant(CapnpUnionVariant::new_group(
            "metadata".to_string(),
            5,
            group2_fields,
        ));

        s.set_union(union);

        let result = s.validate();
        assert!(result.is_err());

        if let Err(ValidationError::DuplicateId { id, locations }) = result {
            assert_eq!(id, 1);
            assert_eq!(locations.len(), 2);
            assert!(locations.contains(&"union group 'dimensions' field 'width'".to_string()));
            assert!(locations.contains(&"union group 'metadata' field 'name'".to_string()));
        } else {
            panic!("Expected DuplicateId error");
        }
    }

    #[test]
    fn test_group_field_duplicate_ids() {
        let mut s = CapnpStruct::new("TestStruct".to_string());
        let mut union = CapnpUnion::new();

        union.add_variant(CapnpUnionVariant::new_group(
            "groupA".to_string(),
            0,
            vec![CapnpField::new("x".to_string(), 1, CapnpType::UInt32)],
        ));
        union.add_variant(CapnpUnionVariant::new_group(
            "groupB".to_string(),
            2,
            vec![
                CapnpField::new("y".to_string(), 0, CapnpType::Text), // Duplicate ID 0 (same as groupA variant ID)
            ],
        ));
        s.set_union(union);

        let err = s.validate().unwrap_err();
        let ValidationError::DuplicateId { id, locations } = err;
        assert_eq!(id, 0);
        assert_eq!(locations.len(), 2);
        assert!(locations.contains(&"union variant 'groupA'".to_string()));
        assert!(locations.contains(&"union group 'groupB' field 'y'".to_string()));
    }

    // Tests for automatic validation during rendering
    #[test]
    fn test_render_validation_failure_struct() {
        let mut s = CapnpStruct::new("InvalidStruct".to_string());
        s.add_field(CapnpField::new("field1".to_string(), 0, CapnpType::UInt32));
        s.add_field(CapnpField::new("field2".to_string(), 0, CapnpType::Text)); // Duplicate ID

        let result = s.render();
        assert!(result.is_err());

        if let Err(ValidationError::DuplicateId { id, locations }) = result {
            assert_eq!(id, 0);
            assert_eq!(locations.len(), 2);
        } else {
            panic!("Expected DuplicateId error during render");
        }
    }

    #[test]
    fn test_render_validation_failure_document() {
        let mut doc = CapnpDocument::new();

        let mut valid_s = CapnpStruct::new("ValidStruct".to_string());
        valid_s.add_field(CapnpField::new("field1".to_string(), 0, CapnpType::UInt32));
        doc.add_item(CapnpItem::Struct(valid_s));

        let mut invalid_s = CapnpStruct::new("InvalidStruct".to_string());
        invalid_s.add_field(CapnpField::new("field1".to_string(), 1, CapnpType::UInt32));
        invalid_s.add_field(CapnpField::new("field2".to_string(), 1, CapnpType::Text)); // Duplicate ID
        doc.add_item(CapnpItem::Struct(invalid_s));

        let result = doc.render();
        assert!(result.is_err());

        if let Err(ValidationError::DuplicateId { id, .. }) = result {
            assert_eq!(id, 1);
        } else {
            panic!("Expected DuplicateId error during document render");
        }
    }
}
