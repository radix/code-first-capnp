use heck::ToLowerCamelCase;
use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Error, Fields, FieldsNamed, FieldsUnnamed, Lit, LitInt, LitStr,
    Result, Token, parse_macro_input,
};

/// Main derive macro for CapnpType
#[proc_macro_derive(CapnpType, attributes(capnp))]
pub fn derive_capnp_type(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match generate_capnp_type(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn generate_capnp_type(input: &DeriveInput) -> Result<proc_macro2::TokenStream> {
    let name = &input.ident;

    // Determine the correct crate name to use
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
            pub fn get_capnp_schema() -> #crate_name::capnp_model::SchemaItem {
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
        #crate_name::capnp_model::SchemaItem::Struct(
            #crate_name::capnp_model::Struct {
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
                            #crate_name::capnp_model::UnionVariantInner::Type {
                                id: #variant_id,
                                capnp_type: #crate_name::capnp_model::CapnpType::Void,
                            }
                        }
                    }
                    Fields::Unnamed(fields) => {
                        // Data-bearing variants become groups - no variant ID needed
                        let group_fields = generate_unnamed_fields(fields, crate_name)?;
                        quote! {
                            #crate_name::capnp_model::UnionVariantInner::Group(
                                vec![#(#group_fields),*]
                            )
                        }
                    }
                    Fields::Named(fields) => {
                        // Named fields - use Group - no variant ID needed
                        let group_fields = generate_named_fields(fields, crate_name)?;
                        quote! {
                            #crate_name::capnp_model::UnionVariantInner::Group(
                                vec![#(#group_fields),*]
                            )
                        }
                    }
                };
                let variant_name_camel = variant_name.to_lower_camel_case();

                variant_tokens.push(quote! {
                    #crate_name::capnp_model::UnionVariant {
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
        #crate_name::capnp_model::SchemaItem::Struct(
            #crate_name::capnp_model::Struct {
                name: #type_name.to_string(),
                fields: vec![],
                union: Some(#crate_name::capnp_model::Union {
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
            #crate_name::capnp_model::Field {
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
            #crate_name::capnp_model::Field {
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
                return Ok(quote! { #crate_name::capnp_model::CapnpType::Text });
            }

            if path.is_ident("bool") {
                return Ok(quote! { #crate_name::capnp_model::CapnpType::Bool });
            }

            // Handle integer types
            if path.is_ident("u8") {
                return Ok(quote! { #crate_name::capnp_model::CapnpType::UInt8 });
            }
            if path.is_ident("u16") {
                return Ok(quote! { #crate_name::capnp_model::CapnpType::UInt16 });
            }
            if path.is_ident("u32") {
                return Ok(quote! { #crate_name::capnp_model::CapnpType::UInt32 });
            }
            if path.is_ident("u64") {
                return Ok(quote! { #crate_name::capnp_model::CapnpType::UInt64 });
            }
            if path.is_ident("i8") {
                return Ok(quote! { #crate_name::capnp_model::CapnpType::Int8 });
            }
            if path.is_ident("i16") {
                return Ok(quote! { #crate_name::capnp_model::CapnpType::Int16 });
            }
            if path.is_ident("i32") {
                return Ok(quote! { #crate_name::capnp_model::CapnpType::Int32 });
            }
            if path.is_ident("i64") {
                return Ok(quote! { #crate_name::capnp_model::CapnpType::Int64 });
            }
            if path.is_ident("f32") {
                return Ok(quote! { #crate_name::capnp_model::CapnpType::Float32 });
            }
            if path.is_ident("f64") {
                return Ok(quote! { #crate_name::capnp_model::CapnpType::Float64 });
            }

            // Handle Vec<T>
            if let Some(segment) = path.segments.first() {
                if segment.ident == "Vec" {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                            let inner_type_tokens =
                                generate_capnp_type_tokens(inner_type, crate_name)?;
                            return Ok(quote! {
                                #crate_name::capnp_model::CapnpType::List(
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
                #crate_name::capnp_model::CapnpType::UserDefined(#type_name.to_string())
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
