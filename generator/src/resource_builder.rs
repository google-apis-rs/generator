use crate::{Resource, method_builder};
use proc_macro2::TokenStream;
use quote::quote;


pub(crate) fn generate(resource: &Resource) -> TokenStream {
    let mod_name = &resource.ident;
    let param_type_defs = resource.methods.iter().flat_map(|method| method.params.iter().filter_map(|param| param.typ.type_def()));
    let method_builders = resource.methods.iter().map(method_builder::generate);
    quote!{mod #mod_name {
        mod params {
            #(#param_type_defs)*
        }
        #(#method_builders)*
    }}
}