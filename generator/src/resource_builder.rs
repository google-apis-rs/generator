use crate::{method_builder, to_ident, to_rust_typestr, to_rust_varstr, ParamInitMethod, Resource};
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;

pub(crate) fn generate(resource: &Resource) -> TokenStream {
    let ident = &resource.ident;
    let param_type_defs = resource.methods.iter().flat_map(|method| {
        method
            .params
            .iter()
            .filter_map(|param| param.typ.type_def())
    });
    let method_builders = resource.methods.iter().map(method_builder::generate);
    let nested_resource_mods = resource.resources.iter().map(generate);

    let method_actions = resource.methods.iter().map(|method| {
        let method_ident = to_ident(&to_rust_varstr(&method.id));
        let method_builder_type = method.builder_name();
        let required_args = method.params.iter().filter(|p| p.required).map(|param| {
            let name = &param.ident;
            let init_method: syn::FnArg = match param.init_method() {
                ParamInitMethod::IntoImpl(into_typ) => parse_quote! {#name: impl Into<#into_typ>},
                ParamInitMethod::ByValue => {
                    let ty = param.typ.type_path();
                    parse_quote! {#name: #ty}
                }
            };
            init_method
        });
        let method_builder_initializers = method.params.iter().map(|param| {
            let name = &param.ident;
            let field_pattern: syn::FieldValue = if param.required {
                match param.init_method() {
                    ParamInitMethod::IntoImpl(_) => parse_quote! {#name: #name.into()},
                    ParamInitMethod::ByValue => parse_quote! {#name},
                }
            } else {
                parse_quote! {#name: None}
            };
            field_pattern
        });
        let method_description = &method.description;
        quote! {
            #[doc = #method_description]
            pub fn #method_ident(&self#(, #required_args)*) -> #method_builder_type {
                #method_builder_type{
                    #(#method_builder_initializers,)*
                }
            }
        }
    });
    let sub_resource_actions = resource.resources.iter().map(|sub_resource| {
        let sub_resource_ident = &sub_resource.ident;
        let sub_action_ident = sub_resource.action_type_name();
        let description = format!(
            "Actions that can be performed on the {} resource",
            sub_resource_ident
        );
        quote! {
            #[doc = #description]
            pub fn #sub_resource_ident(&self) -> #sub_resource_ident::#sub_action_ident {
                #sub_resource_ident::#sub_action_ident
            }
        }
    });
    let action_ident = resource.action_type_name();
    quote! {
        pub mod #ident {
            pub mod params {
                #(#param_type_defs)*
            }

            pub struct #action_ident;
            impl #action_ident {
                #(#method_actions)*
                #(#sub_resource_actions)*
            }

            #(#method_builders)*
            #(#nested_resource_mods)*
        }
    }
}
