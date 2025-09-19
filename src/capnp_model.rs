//! Abstract document model for Cap'n Proto schemas.
//!
//! This module defines data structures that represent Cap'n Proto schemas
//! in an abstract way, separate from the string generation logic.

use std::fmt::Write;

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
    pub variant_type: CapnpType,
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
        Self {
            items: Vec::new(),
        }
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

    /// Renders the document as Cap'n Proto schema text
    pub fn render(&self) -> String {
        let mut output = String::new();

        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                writeln!(&mut output).unwrap();
            }
            write!(&mut output, "{}", item.render()).unwrap();
        }

        output
    }
}

impl CapnpItem {
    /// Renders the item as Cap'n Proto schema text
    pub fn render(&self) -> String {
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

    /// Renders the struct as Cap'n Proto schema text
    pub fn render(&self) -> String {
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

        output
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
    /// Creates a new union variant
    pub fn new(name: String, id: u32, variant_type: CapnpType) -> Self {
        Self {
            name,
            id,
            variant_type,
        }
    }

    /// Renders the variant as Cap'n Proto schema text
    pub fn render(&self) -> String {
        format!("{} @{} :{};", self.name, self.id, self.variant_type.render())
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
        assert_eq!(doc.render(), "");
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

        let output = doc.render();

        assert!(output.contains("struct Person {"));
        assert!(output.contains("struct Company {"));
        // Test that there's spacing between structs
        assert!(output.contains("}\n\nstruct Company"));
    }

    // Struct tests
    #[test]
    fn test_empty_struct() {
        let s = CapnpStruct::new("Empty".to_string());
        let output = s.render();

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
        union.add_variant(CapnpUnionVariant::new("variant".to_string(), 1, CapnpType::Void));
        s.set_union(union);

        let output = s.render();

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

        let deeply_nested = CapnpType::List(Box::new(CapnpType::List(Box::new(CapnpType::List(Box::new(CapnpType::Bool))))));
        assert_eq!(deeply_nested.render(), "List(List(List(Bool)))");
    }

    // Integration tests
    #[test]
    fn test_simple_struct_rendering() {
        let mut s = CapnpStruct::new("Person".to_string());
        s.add_field(CapnpField::new("id".to_string(), 0, CapnpType::UInt64));
        s.add_field(CapnpField::new("name".to_string(), 1, CapnpType::Text));

        let doc = CapnpDocument::with_struct(s);
        let output = doc.render();

        assert!(output.contains("struct Person {"));
        assert!(output.contains("id @0 :UInt64;"));
        assert!(output.contains("name @1 :Text;"));
        assert!(output.contains("}"));
    }

    #[test]
    fn test_union_struct_rendering() {
        let mut s = CapnpStruct::new("Message".to_string());

        let mut union = CapnpUnion::new();
        union.add_variant(CapnpUnionVariant::new("text".to_string(), 0, CapnpType::Text));
        union.add_variant(CapnpUnionVariant::new("number".to_string(), 1, CapnpType::UInt32));

        s.set_union(union);

        let doc = CapnpDocument::with_struct(s);
        let output = doc.render();

        assert!(output.contains("struct Message {"));
        assert!(output.contains("union {"));
        assert!(output.contains("text @0 :Text;"));
        assert!(output.contains("number @1 :UInt32;"));
        assert!(output.contains("  }"));
        assert!(output.contains("}"));
    }

    #[test]
    fn test_complex_struct_with_all_types() {
        let mut s = CapnpStruct::new("ComplexStruct".to_string());

        // Add fields with various types
        s.add_field(CapnpField::new("boolField".to_string(), 0, CapnpType::Bool));
        s.add_field(CapnpField::new("intField".to_string(), 1, CapnpType::Int32));
        s.add_field(CapnpField::new("floatField".to_string(), 2, CapnpType::Float64));
        s.add_field(CapnpField::new("textField".to_string(), 3, CapnpType::Text));
        s.add_field(CapnpField::new("listField".to_string(), 4, CapnpType::List(Box::new(CapnpType::UInt32))));
        s.add_field(CapnpField::new("customField".to_string(), 5, CapnpType::UserDefined("CustomType".to_string())));

        // Add union
        let mut union = CapnpUnion::new();
        union.add_variant(CapnpUnionVariant::new("voidVariant".to_string(), 6, CapnpType::Void));
        union.add_variant(CapnpUnionVariant::new("textVariant".to_string(), 7, CapnpType::Text));
        s.set_union(union);

        let output = s.render();

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

}
