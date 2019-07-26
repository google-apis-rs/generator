use discovery_parser::{normalize, RawDiscoveryRestDesc, RefOrType, SchemaDesc, TypeDesc};
use generator::{to_type_ident, to_var_ident, to_variant_ident};
use proc_macro2::TokenStream;
use quote::quote;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    //let url = "https://www.googleapis.com/discovery/v1/apis/compute/v1/rest";
    let url = "https://www.googleapis.com/discovery/v1/apis/drive/v3/rest";
    //println!("Fetching {}", url);
    let body: String = reqwest::get(url)?.text()?;
    std::fs::write("/tmp/content", &body)?;
    let desc: RawDiscoveryRestDesc = serde_json::from_str(&body)?;
    //println!("raw: {:#?}", desc);
    //println!("{:#?}", desc);
    let desc = normalize(desc);
    //println!("normalized: {:#?}", desc);

    for schema in desc.schemas.values() {
        println!("{}", typedef_for_schema(schema));
    }
    Ok(())
}

fn typedef_for_schema(schema: &SchemaDesc) -> TokenStream {
    let name = to_type_ident(&schema.id);

    match schema.typ {
        TypeDesc::Any => {
            quote! { type #name = json::Value; }
        }
        TypeDesc::String => {
            quote! { type #name = String;}
        }
        TypeDesc::FormattedString { .. } => {
            quote! { type #name = String; }
        }
        TypeDesc::Boolean => {
            quote! { type #name = bool; }
        }
        TypeDesc::Int32 => {
            quote! { type #name = i32; }
        }
        TypeDesc::Uint32 => {
            quote! { type #name = u32; }
        }
        TypeDesc::Float64 => {
            quote! { type #name = f64; }
        }
        TypeDesc::Float32 => {
            quote! { type #name = f32; }
        }
        TypeDesc::Bytes => {
            quote! { type #name = Vec<u8>; }
        }
        TypeDesc::Date => {
            quote! { type #name = String; }
        }
        TypeDesc::DateTime => {
            quote! { type #name = String; }
        }
        TypeDesc::Int64 => {
            quote! { type #name = i64; }
        }
        TypeDesc::Uint64 => {
            quote! { type #name = u64; }
        }
        TypeDesc::Enumeration(ref enums) => {
            let variants = enums
                .iter()
                .map(|enum_desc| to_variant_ident(&enum_desc.value));
            quote! {
                #[derive(Debug,Clone,Copy,Deserialize)]
                pub enum #name {
                    #(#variants,)*
                }
            }
        }
        TypeDesc::Array { ref items } => {
            let items_type = struct_field_type(&items);
            quote! { type #name = Vec<#items_type>; }
        }
        TypeDesc::Object {
            ref properties,
            ref additional_properties,
        } => match (properties.is_empty(), additional_properties) {
            (true, None) => panic!("object without properties or additional properties"),
            (true, Some(boxed_prop_desc)) => {
                let prop_type = struct_field_type(&boxed_prop_desc.typ);
                quote! { HashMap<String, #prop_type> }
            }
            (false, additional_properties) => {
                let struct_fields: Vec<_> = properties
                    .iter()
                    .map(|(name, prop_type)| {
                        let field_name = to_var_ident(&name);
                        let typ = struct_field_type(&prop_type.typ);
                        quote! {
                            #field_name: Option<#typ>
                        }
                    })
                    .collect();
                let additional_properties_field =
                    if let Some(boxed_prop_desc) = additional_properties {
                        let prop_type = struct_field_type(&boxed_prop_desc.typ);
                        quote! {
                            #[serde(default,flatten)]
                            additional_properties: HashMap<String, #prop_type>,
                        }
                    } else {
                        quote! {}
                    };
                quote! {
                    #[derive(Debug,Clone,Deserialize)]
                    pub struct #name {
                        #(pub #struct_fields,)*
                        #additional_properties_field
                    }
                }
            }
        },
    }
}

fn struct_field_type(prop_type: &RefOrType<TypeDesc>) -> TokenStream {
    match prop_type {
        RefOrType::Ref(ref_type) => {
            let ident = to_type_ident(ref_type);
            quote! { #ident }
        }
        RefOrType::Type(type_desc) => match type_desc {
            TypeDesc::Any => quote! { TODO-Any },
            TypeDesc::String => quote! { String },
            TypeDesc::FormattedString { .. } => quote! { String },
            TypeDesc::Boolean => quote! { bool },
            TypeDesc::Int32 => quote! { i32 },
            TypeDesc::Uint32 => quote! { u32 },
            TypeDesc::Float64 => quote! { f64 },
            TypeDesc::Float32 => quote! { f32 },
            TypeDesc::Bytes => quote! { Vec<u8> },
            TypeDesc::Date => quote! { Date<UTC> },
            TypeDesc::DateTime => quote! { DateTime<UTC> },
            TypeDesc::Int64 => quote! { i64 },
            TypeDesc::Uint64 => quote! { u64 },
            TypeDesc::Enumeration(_) => panic!("this should be a ref after normalizing"),
            TypeDesc::Array { items } => {
                let items_type = struct_field_type(&items);
                quote! { Vec<#items_type> }
            }
            TypeDesc::Object {
                properties,
                additional_properties,
            } => match (properties.is_empty(), additional_properties.is_none()) {
                (true, true) => panic!("object without properties of additional_properties"),
                (true, false) => {
                    let additional_properties = additional_properties.as_ref().unwrap();
                    let property_type = struct_field_type(&additional_properties.typ);
                    quote! { HashMap<String, #property_type> }
                }
                (false, true) => {
                    panic!("this should be a ref after normalizing");
                }
                (false, false) => {
                    panic!("this should be a ref after normalizing");
                }
            },
        },
    }
}
