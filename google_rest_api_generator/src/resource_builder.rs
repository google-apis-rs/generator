use crate::{method_actions, method_builder, Param, Resource, Type, RefOrType};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::BTreeMap;
use std::borrow::Cow;

pub(crate) fn generate(
    root_url: &str,
    service_path: &str,
    global_params: &[Param],
    resource: &Resource,
    schemas: &BTreeMap<syn::Ident, Type>,
) -> TokenStream {
    let ident = &resource.ident;
    let mut param_type_defs = Vec::new();
    for param in resource.methods.iter().flat_map(|method| method.params.iter()) {
        crate::append_nested_type_defs(
            &RefOrType::Type(Cow::Borrowed(&param.typ)),
            schemas,
            &mut param_type_defs,
        );
    }
    let method_builders = resource.methods.iter().map(|method| {
        method_builder::generate(root_url, service_path, global_params, method, schemas)
    });
    let nested_resource_mods = resource
        .resources
        .iter()
        .map(|resource| generate(root_url, service_path, global_params, resource, schemas));

    let method_actions = resource
        .methods
        .iter()
        .map(|method| method_actions::generate(method, global_params));
    let nested_resource_actions = resource
        .resources
        .iter()
        .map(|sub_resource| crate::resource_actions::generate(sub_resource));
    let action_ident = resource.action_type_name();
    quote! {
        pub mod #ident {
            pub mod params {
                #(#param_type_defs)*
            }

            pub struct #action_ident<'a, A> {
                pub(crate) reqwest: &'a reqwest::Client,
                pub(crate) auth: &'a std::sync::Mutex<A>,
            }
            impl<'a, A: yup_oauth2::GetToken> #action_ident<'a, A> {
                #(#method_actions)*
                #(#nested_resource_actions)*
            }

            #(#method_builders)*
            #(#nested_resource_mods)*
        }
    }
}
