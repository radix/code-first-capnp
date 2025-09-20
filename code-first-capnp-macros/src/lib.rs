use heck::ToLowerCamelCase;
use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::Mutex;
use syn::{
    Attribute, Data, DeriveInput, Error, Fields, FieldsNamed, FieldsUnnamed, Lit, LitInt, LitStr,
    Result, Token, parse_macro_input,
};

// Global state to track schema files and their content
static SCHEMA_FILES: LazyLock<Mutex<HashMap<String, (u64, Vec<capnp_model::SchemaItem>)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Creates a new capnp schema file and initializes it with the file ID
#[proc_macro]
pub fn capnp_schema_file(input: TokenStream) -> TokenStream {
    let parser = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
    let input = parse_macro_input!(input with parser);
    let mut iter = input.into_iter();

    let filename = match iter.next() {
        Some(syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        })) => s.value(),
        _ => {
            return syn::Error::new(Span::call_site(), "First argument must be a string literal")
                .to_compile_error()
                .into();
        }
    };

    let file_id = match iter.next() {
        Some(syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(i),
            ..
        })) => match i.base10_parse::<u64>() {
            Ok(id) => id,
            Err(_) => {
                return syn::Error::new(Span::call_site(), "Second argument must be a valid u64")
                    .to_compile_error()
                    .into();
            }
        },
        _ => {
            return syn::Error::new(
                Span::call_site(),
                "Second argument must be an integer literal",
            )
            .to_compile_error()
            .into();
        }
    };

    // Initialize the schema file in our global state
    let mut files = SCHEMA_FILES.lock().unwrap();
    files.insert(filename.clone(), (file_id, Vec::new()));

    // The macro expands to nothing visible in the code
    quote!().into()
}

/// Main derive macro for CapnpType - now also appends to schema files
#[proc_macro_derive(CapnpType, attributes(capnp))]
pub fn derive_capnp_type(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match generate_capnp_type(&input) {
        Ok(tokens) => {
            // Check if this type should be added to a schema file
            if let Ok(filename) = extract_schema_filename(&input.attrs) {
                if let Err(e) = record_schema_item(&input, &filename) {
                    return e.to_compile_error().into();
                }
            }
            tokens.into()
        }
        Err(err) => err.to_compile_error().into(),
    }
}

/// Completes the capnp schema compilation and generates the Rust code
#[proc_macro]
pub fn complete_capnp_schema(input: TokenStream) -> TokenStream {
    let input_tokens = proc_macro2::TokenStream::from(input);
    let mut tokens = input_tokens.into_iter();

    // Parse the filename string literal
    let filename = match tokens.next() {
        Some(proc_macro2::TokenTree::Literal(lit)) => {
            match syn::parse2::<syn::LitStr>(proc_macro2::TokenStream::from(
                proc_macro2::TokenTree::Literal(lit),
            )) {
                Ok(lit_str) => lit_str.value(),
                Err(_) => {
                    return syn::Error::new(
                        Span::call_site(),
                        "First argument must be a string literal",
                    )
                    .to_compile_error()
                    .into();
                }
            }
        }
        _ => {
            return syn::Error::new(Span::call_site(), "First argument must be a string literal")
                .to_compile_error()
                .into();
        }
    };

    // Skip comma
    match tokens.next() {
        Some(proc_macro2::TokenTree::Punct(punct)) if punct.as_char() == ',' => {}
        _ => {
            return syn::Error::new(Span::call_site(), "Expected comma after filename")
                .to_compile_error()
                .into();
        }
    }

    // Collect the rest as the module declaration
    let module_decl_tokens: proc_macro2::TokenStream = tokens.collect();

    // Get the accumulated schema content and write it all at once
    let (file_id, schema_items) = {
        let files = SCHEMA_FILES.lock().unwrap();
        match files.get(&filename) {
            Some((file_id, items)) => (*file_id, items.clone()),
            None => {
                return syn::Error::new(
                    Span::call_site(),
                    &format!(
                        "No schema file '{}' found. Did you call capnp_schema_file! first?",
                        filename
                    ),
                )
                .to_compile_error()
                .into();
            }
        }
    };

    // Write the schema to the manifest directory
    let manifest_dir =
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR environment variable not set");
    let manifest_dir = PathBuf::from(manifest_dir);
    let schema_path = manifest_dir.join(&filename);

    // Create the complete schema
    let mut schema = capnp_model::Schema::new();
    for item in &schema_items {
        schema.add_item(item.clone());
    }

    // Render and write the schema
    let schema_content = match schema.render() {
        Ok(content) => content,
        Err(e) => {
            return syn::Error::new(
                Span::call_site(),
                &format!("Failed to render schema: {}", e),
            )
            .to_compile_error()
            .into();
        }
    };

    let full_content = format!("@0x{:x};\n\n{}", file_id, schema_content);

    if let Err(e) = fs::write(&schema_path, full_content) {
        return syn::Error::new(
            Span::call_site(),
            &format!("Failed to write schema file: {}", e),
        )
        .to_compile_error()
        .into();
    }

    // Use capnpc to compile the schema
    if let Err(e) = capnpc::CompilerCommand::new()
        .src_prefix(&manifest_dir)
        .file(&schema_path)
        .output_path(&manifest_dir)
        .run()
    {
        return syn::Error::new(
            Span::call_site(),
            &format!("Failed to compile schema with capnpc: {}", e),
        )
        .to_compile_error()
        .into();
    }

    // Generate a manual include of the generated file
    let generated_file = format!("../{}_capnp.rs", filename.trim_end_matches(".capnp"));
    quote! {
        #module_decl_tokens {
            include!(#generated_file);
        }
    }
    .into()
}

fn record_schema_item(input: &DeriveInput, filename: &str) -> Result<()> {
    // Generate schema item using the capnp_model
    let schema_item = generate_schema_item_with_model(input)?;

    // Add to the global state
    let mut files = SCHEMA_FILES.lock().unwrap();
    if let Some((_, items)) = files.get_mut(filename) {
        items.push(schema_item);
    } else {
        return Err(Error::new(
            Span::call_site(),
            &format!(
                "Schema file '{}' not initialized. Did you call capnp_schema_file! first?",
                filename
            ),
        ));
    }

    Ok(())
}

fn generate_schema_item_with_model(input: &DeriveInput) -> Result<capnp_model::SchemaItem> {
    // Create the appropriate SchemaItem using capnp_model
    match &input.data {
        Data::Struct(_) => generate_struct_schema_item(&input),
        Data::Enum(_) => generate_enum_schema_item(&input),
        Data::Union(_) => Err(Error::new_spanned(input, "Union types are not supported")),
    }
}

fn generate_struct_schema_item(input: &DeriveInput) -> Result<capnp_model::SchemaItem> {
    let name = input.ident.to_string();
    let mut struct_def = capnp_model::Struct::new(name);

    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => generate_named_fields_for_model(fields)?,
            Fields::Unnamed(fields) => generate_unnamed_fields_for_model(fields)?,
            Fields::Unit => Vec::new(),
        },
        _ => unreachable!(),
    };

    for field in fields {
        struct_def.add_field(field);
    }

    // Add extra fields
    let extra_fields = extract_extra_fields(&input.attrs)?;
    for extra in extra_fields {
        struct_def.add_extra_field(extra);
    }

    Ok(capnp_model::SchemaItem::Struct(struct_def))
}

fn generate_enum_schema_item(input: &DeriveInput) -> Result<capnp_model::SchemaItem> {
    let name = input.ident.to_string();
    let mut struct_def = capnp_model::Struct::new(name);
    let mut union_def = capnp_model::Union::new();

    let _variants = match &input.data {
        Data::Enum(data_enum) => {
            for variant in &data_enum.variants {
                let variant_name = variant.ident.to_string().to_lower_camel_case();

                let union_variant = match &variant.fields {
                    Fields::Unit => {
                        let variant_id = extract_capnp_id(&variant.attrs)?;
                        capnp_model::UnionVariant::new(
                            variant_name,
                            variant_id,
                            capnp_model::CapnpType::Void,
                        )
                    }
                    Fields::Unnamed(fields) => {
                        let group_fields = generate_unnamed_fields_for_model(fields)?;
                        capnp_model::UnionVariant::new_group(variant_name, group_fields)
                    }
                    Fields::Named(fields) => {
                        let group_fields = generate_named_fields_for_model(fields)?;
                        capnp_model::UnionVariant::new_group(variant_name, group_fields)
                    }
                };

                union_def.add_variant(union_variant);
            }
        }
        _ => unreachable!(),
    };

    struct_def.set_union(union_def);

    // Add extra fields
    let extra_fields = extract_extra_fields(&input.attrs)?;
    for extra in extra_fields {
        struct_def.add_extra_field(extra);
    }

    Ok(capnp_model::SchemaItem::Struct(struct_def))
}

fn generate_named_fields_for_model(fields: &FieldsNamed) -> Result<Vec<capnp_model::Field>> {
    let mut result = Vec::new();

    for field in &fields.named {
        let field_name = field.ident.as_ref().unwrap().to_string();
        let field_id = extract_capnp_id(&field.attrs)?;
        let custom_name = extract_custom_name(&field.attrs)?;
        let capnp_name = custom_name.unwrap_or_else(|| field_name.to_lower_camel_case());
        let field_type = rust_type_to_capnp_model_type(&field.ty)?;

        result.push(capnp_model::Field::new(capnp_name, field_id, field_type));
    }

    Ok(result)
}

fn generate_unnamed_fields_for_model(fields: &FieldsUnnamed) -> Result<Vec<capnp_model::Field>> {
    let mut result = Vec::new();

    for (index, field) in fields.unnamed.iter().enumerate() {
        let field_name = format!("field{}", index);
        let field_id = extract_capnp_id(&field.attrs)?;
        let field_type = rust_type_to_capnp_model_type(&field.ty)?;

        result.push(capnp_model::Field::new(field_name, field_id, field_type));
    }

    Ok(result)
}

fn rust_type_to_capnp_model_type(ty: &syn::Type) -> Result<capnp_model::CapnpType> {
    match ty {
        syn::Type::Path(type_path) => {
            let path = &type_path.path;

            // Handle common types
            if path.is_ident("String") {
                return Ok(capnp_model::CapnpType::Text);
            }
            if path.is_ident("bool") {
                return Ok(capnp_model::CapnpType::Bool);
            }

            // Handle integer types
            if path.is_ident("u8") {
                return Ok(capnp_model::CapnpType::UInt8);
            }
            if path.is_ident("u16") {
                return Ok(capnp_model::CapnpType::UInt16);
            }
            if path.is_ident("u32") {
                return Ok(capnp_model::CapnpType::UInt32);
            }
            if path.is_ident("u64") {
                return Ok(capnp_model::CapnpType::UInt64);
            }
            if path.is_ident("i8") {
                return Ok(capnp_model::CapnpType::Int8);
            }
            if path.is_ident("i16") {
                return Ok(capnp_model::CapnpType::Int16);
            }
            if path.is_ident("i32") {
                return Ok(capnp_model::CapnpType::Int32);
            }
            if path.is_ident("i64") {
                return Ok(capnp_model::CapnpType::Int64);
            }
            if path.is_ident("f32") {
                return Ok(capnp_model::CapnpType::Float32);
            }
            if path.is_ident("f64") {
                return Ok(capnp_model::CapnpType::Float64);
            }

            // Handle Vec<T>
            if let Some(segment) = path.segments.first() {
                if segment.ident == "Vec" {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                            let inner_capnp_type = rust_type_to_capnp_model_type(inner_type)?;
                            return Ok(capnp_model::CapnpType::List(Box::new(inner_capnp_type)));
                        }
                    }
                }
            }

            // Handle user-defined types
            let type_name = path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");

            Ok(capnp_model::CapnpType::UserDefined(type_name))
        }
        _ => Err(Error::new_spanned(ty, "Unsupported type")),
    }
}

fn generate_capnp_type(input: &DeriveInput) -> Result<proc_macro2::TokenStream> {
    let name = &input.ident;

    // Determine the correct crate name to use -- this is really only to support unit tests in the
    // code-first-capnp crate.
    let crate_name = match crate_name("code-first-capnp") {
        Ok(FoundCrate::Itself) => quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = proc_macro2::Ident::new(&name, proc_macro2::Span::call_site());
            quote!(#ident)
        }
        Err(_) => quote!(code_first_capnp),
    };

    let schema_item = match &input.data {
        Data::Struct(_) => generate_struct_schema(&input, &crate_name)?,
        Data::Enum(_) => generate_enum_schema(&input, &crate_name)?,
        Data::Union(_) => {
            return Err(Error::new_spanned(input, "Union types are not supported"));
        }
    };

    Ok(quote! {
        impl #name {
            pub fn get_capnp_schema() -> #crate_name::SchemaItem {
                #schema_item
            }
        }
    })
}

fn generate_struct_schema(
    input: &DeriveInput,
    crate_name: &proc_macro2::TokenStream,
) -> Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let type_name = name.to_string();

    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => generate_named_fields(fields, crate_name)?,
            Fields::Unnamed(fields) => generate_unnamed_fields(fields, crate_name)?,
            Fields::Unit => Vec::new(),
        },
        _ => unreachable!(),
    };

    let extra_fields = extract_extra_fields(&input.attrs)?;

    let fields_tokens = if fields.is_empty() {
        quote! { vec![] }
    } else {
        quote! { vec![#(#fields),*] }
    };

    let extra_fields_tokens = if extra_fields.is_empty() {
        quote! { vec![] }
    } else {
        let extra_strs: Vec<_> = extra_fields.iter().collect();
        quote! { vec![#(#extra_strs.to_string()),*] }
    };

    Ok(quote! {
        #crate_name::SchemaItem::Struct(
            #crate_name::Struct {
                name: #type_name.to_string(),
                fields: #fields_tokens,
                union: None,
                extra_fields: #extra_fields_tokens,
            }
        )
    })
}

fn generate_enum_schema(
    input: &DeriveInput,
    crate_name: &proc_macro2::TokenStream,
) -> Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let type_name = name.to_string();

    let variants = match &input.data {
        Data::Enum(data_enum) => {
            let mut variant_tokens = Vec::new();

            for variant in &data_enum.variants {
                let variant_name = variant.ident.to_string();

                let variant_inner = match &variant.fields {
                    Fields::Unit => {
                        // Unit variants require an ID on the variant itself
                        let variant_id = extract_capnp_id(&variant.attrs)?;
                        quote! {
                            #crate_name::UnionVariantInner::Type {
                                id: #variant_id,
                                capnp_type: #crate_name::CapnpType::Void,
                            }
                        }
                    }
                    Fields::Unnamed(fields) => {
                        // Data-bearing variants become groups - no variant ID needed
                        let group_fields = generate_unnamed_fields(fields, crate_name)?;
                        quote! {
                            #crate_name::UnionVariantInner::Group(
                                vec![#(#group_fields),*]
                            )
                        }
                    }
                    Fields::Named(fields) => {
                        // Named fields - use Group - no variant ID needed
                        let group_fields = generate_named_fields(fields, crate_name)?;
                        quote! {
                            #crate_name::UnionVariantInner::Group(
                                vec![#(#group_fields),*]
                            )
                        }
                    }
                };
                let variant_name_camel = variant_name.to_lower_camel_case();

                variant_tokens.push(quote! {
                    #crate_name::UnionVariant {
                        name: #variant_name_camel.to_string(),
                        variant_inner: #variant_inner,
                    }
                });
            }

            variant_tokens
        }
        _ => unreachable!(),
    };

    let extra_fields = extract_extra_fields(&input.attrs)?;
    let extra_fields_tokens = if extra_fields.is_empty() {
        quote! { vec![] }
    } else {
        let extra_strs: Vec<_> = extra_fields.iter().collect();
        quote! { vec![#(#extra_strs.to_string()),*] }
    };

    Ok(quote! {
        #crate_name::SchemaItem::Struct(
            #crate_name::Struct {
                name: #type_name.to_string(),
                fields: vec![],
                union: Some(#crate_name::Union {
                    variants: vec![#(#variants),*],
                }),
                extra_fields: #extra_fields_tokens,
            }
        )
    })
}

fn generate_named_fields(
    fields: &FieldsNamed,
    crate_name: &proc_macro2::TokenStream,
) -> Result<Vec<proc_macro2::TokenStream>> {
    let mut field_tokens = Vec::new();

    for field in &fields.named {
        let field_name = field.ident.as_ref().unwrap().to_string();
        let field_id = extract_capnp_id(&field.attrs)?;
        let custom_name = extract_custom_name(&field.attrs)?;
        let capnp_name = custom_name.unwrap_or_else(|| field_name.to_lower_camel_case());
        let field_type = generate_capnp_type_tokens(&field.ty, crate_name)?;

        field_tokens.push(quote! {
            #crate_name::CapnpField {
                name: #capnp_name.to_string(),
                id: #field_id,
                field_type: #field_type,
            }
        });
    }

    Ok(field_tokens)
}

fn generate_unnamed_fields(
    fields: &FieldsUnnamed,
    crate_name: &proc_macro2::TokenStream,
) -> Result<Vec<proc_macro2::TokenStream>> {
    let mut field_tokens = Vec::new();

    for (index, field) in fields.unnamed.iter().enumerate() {
        let field_name = format!("field{}", index);
        let field_id = extract_capnp_id(&field.attrs)?;
        let field_type = generate_capnp_type_tokens(&field.ty, crate_name)?;

        field_tokens.push(quote! {
            #crate_name::CapnpField {
                name: #field_name.to_string(),
                id: #field_id,
                field_type: #field_type,
            }
        });
    }

    Ok(field_tokens)
}

fn generate_capnp_type_tokens(
    ty: &syn::Type,
    crate_name: &proc_macro2::TokenStream,
) -> Result<proc_macro2::TokenStream> {
    match ty {
        syn::Type::Path(type_path) => {
            let path = &type_path.path;

            // Handle common types
            if path.is_ident("String") {
                return Ok(quote! { #crate_name::CapnpType::Text });
            }

            if path.is_ident("bool") {
                return Ok(quote! { #crate_name::CapnpType::Bool });
            }

            // Handle integer types
            if path.is_ident("u8") {
                return Ok(quote! { #crate_name::CapnpType::UInt8 });
            }
            if path.is_ident("u16") {
                return Ok(quote! { #crate_name::CapnpType::UInt16 });
            }
            if path.is_ident("u32") {
                return Ok(quote! { #crate_name::CapnpType::UInt32 });
            }
            if path.is_ident("u64") {
                return Ok(quote! { #crate_name::CapnpType::UInt64 });
            }
            if path.is_ident("i8") {
                return Ok(quote! { #crate_name::CapnpType::Int8 });
            }
            if path.is_ident("i16") {
                return Ok(quote! { #crate_name::CapnpType::Int16 });
            }
            if path.is_ident("i32") {
                return Ok(quote! { #crate_name::CapnpType::Int32 });
            }
            if path.is_ident("i64") {
                return Ok(quote! { #crate_name::CapnpType::Int64 });
            }
            if path.is_ident("f32") {
                return Ok(quote! { #crate_name::CapnpType::Float32 });
            }
            if path.is_ident("f64") {
                return Ok(quote! { #crate_name::CapnpType::Float64 });
            }

            // Handle Vec<T>
            if let Some(segment) = path.segments.first() {
                if segment.ident == "Vec" {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                            let inner_type_tokens =
                                generate_capnp_type_tokens(inner_type, crate_name)?;
                            return Ok(quote! {
                                #crate_name::CapnpType::List(
                                    Box::new(#inner_type_tokens)
                                )
                            });
                        }
                    }
                }
            }

            // Handle user-defined types
            let type_name = path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");

            Ok(quote! {
                #crate_name::CapnpType::UserDefined(#type_name.to_string())
            })
        }
        _ => Err(Error::new_spanned(ty, "Unsupported type")),
    }
}

fn extract_capnp_id(attrs: &[Attribute]) -> Result<u32> {
    for attr in attrs {
        if attr.path().is_ident("capnp") {
            let mut id: Option<u32> = None;
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("id") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Int(lit_int) = lit {
                        id = Some(lit_int.base10_parse()?);
                    }
                } else {
                    // Skip other attributes
                    if meta.input.peek(syn::Token![=]) {
                        let _: Token![=] = meta.input.parse()?;
                        let _: LitStr = meta.input.parse()?;
                    }
                }
                Ok(())
            });
            if let Some(id) = id {
                return Ok(id);
            }
        }
    }
    Err(Error::new(
        Span::call_site(),
        "Missing required capnp:id attribute",
    ))
}

fn extract_custom_name(attrs: &[Attribute]) -> Result<Option<String>> {
    for attr in attrs {
        if attr.path().is_ident("capnp") {
            let mut name: Option<String> = None;
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(lit_str) = lit {
                        name = Some(lit_str.value());
                    }
                } else {
                    // Skip other attributes
                    if meta.input.peek(syn::Token![=]) {
                        let _: Token![=] = meta.input.parse()?;
                        if meta.path.is_ident("id") {
                            let _: LitInt = meta.input.parse()?;
                        } else {
                            let _: LitStr = meta.input.parse()?;
                        }
                    }
                }
                Ok(())
            });
            if name.is_some() {
                return Ok(name);
            }
        }
    }
    Ok(None)
}

fn extract_extra_fields(attrs: &[Attribute]) -> Result<Vec<String>> {
    let mut extra_fields = Vec::new();

    for attr in attrs {
        if attr.path().is_ident("capnp") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("extra") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(lit_str) = lit {
                        extra_fields.push(lit_str.value());
                    }
                } else {
                    // Skip other attributes
                    if meta.input.peek(syn::Token![=]) {
                        let _: Token![=] = meta.input.parse()?;
                        if meta.path.is_ident("id") {
                            let _: LitInt = meta.input.parse()?;
                        } else {
                            let _: LitStr = meta.input.parse()?;
                        }
                    }
                }
                Ok(())
            });
        }
    }

    Ok(extra_fields)
}

fn extract_schema_filename(attrs: &[Attribute]) -> Result<String> {
    for attr in attrs {
        if attr.path().is_ident("capnp") {
            let mut filename: Option<String> = None;
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("file") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(lit_str) = lit {
                        filename = Some(lit_str.value());
                    }
                } else {
                    // Skip other attributes
                    if meta.input.peek(syn::Token![=]) {
                        let _: Token![=] = meta.input.parse()?;
                        if meta.path.is_ident("id") {
                            let _: LitInt = meta.input.parse()?;
                        } else {
                            let _: LitStr = meta.input.parse()?;
                        }
                    }
                }
                Ok(())
            });
            if let Some(filename) = filename {
                return Ok(filename);
            }
        }
    }
    Err(Error::new(
        Span::call_site(),
        "No capnp file attribute found",
    ))
}
