use crate::{method_actions, method_builder, Param, Resource};
use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn generate(
    root_url: &str,
    service_path: &str,
    global_params: &[Param],
    resource: &Resource,
) -> TokenStream {
    let ident = &resource.ident;
    let param_type_defs = resource.methods.iter().flat_map(|method| {
        method
            .params
            .iter()
            .filter_map(|param| param.typ.type_def())
    });
    let method_builders = resource
        .methods
        .iter()
        .map(|method| method_builder::generate(root_url, service_path, global_params, method));
    let nested_resource_mods = resource
        .resources
        .iter()
        .map(|resource| generate(root_url, service_path, global_params, resource));

    let method_actions = resource
        .methods
        .iter()
        .map(|method| method_actions::generate(method, global_params));
    let sub_resource_actions = resource.resources.iter().map(|sub_resource| {
        let sub_resource_ident = &sub_resource.ident;
        let sub_action_ident = sub_resource.action_type_name();
        let description = format!(
            "Actions that can be performed on the {} resource",
            sub_resource_ident
        );
        quote! {
            #[doc = #description]
            pub fn #sub_resource_ident(&self) -> #sub_resource_ident::#sub_action_ident<A> {
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

            pub struct #action_ident<'a, A> {
                pub(super) reqwest: &'a reqwest::Client,
                pub(super) auth: &'a std::sync::Mutex<A>,
            }
            impl<'a, A: yup_oauth2::GetToken> #action_ident<'a, A> {
                #(#method_actions)*
                #(#sub_resource_actions)*
            }

            #(#method_builders)*
            #(#nested_resource_mods)*
        }
    }
}
