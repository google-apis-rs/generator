use crate::{method_actions, method_builder, Param, Resource, Type};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::BTreeMap;

pub(crate) fn generate(
    root_url: &str,
    service_path: &str,
    global_params: &[Param],
    resource: &Resource,
    schemas: &BTreeMap<syn::Ident, Type>,
) -> TokenStream {
    let ident = &resource.ident;
    let param_type_defs = resource
        .methods
        .iter()
        .flat_map(|method| method.params.iter())
        .fold(Vec::new(), |accum, param| {
            param.typ.fold_nested(accum, |mut accum, typ| {
                if let Some(type_def) = typ.type_def(schemas) {
                    accum.push(type_def);
                }
                accum
            })
        });
    let method_builders = resource.methods.iter().map(|method| {
        method_builder::generate(
            root_url,
            service_path,
            global_params,
            method,
            &resource.action_type_name(),
            schemas,
        )
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

            pub struct #action_ident<'a> {
                pub(crate) reqwest: &'a reqwest::Client,
                pub(crate) auth: &'a dyn ::google_api_auth::GetAccessToken,
            }
            impl<'a> #action_ident<'a> {
                fn auth_ref(&self) -> &dyn ::google_api_auth::GetAccessToken {
                    self.auth
                }

                #(#method_actions)*
                #(#nested_resource_actions)*
            }

            #(#method_builders)*
            #(#nested_resource_mods)*
        }
    }
}
