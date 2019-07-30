use crate::{Resource, method_builder, to_ident, to_rust_varstr, to_rust_typestr};
use proc_macro2::TokenStream;
use quote::quote;

/// The method of initialization used by params. Some are by value (bool, i32,
/// etc) and others are by Into<T> (Into<String>, etc.)
enum InitMethod {
    IntoImpl,
    ByValue,
}

pub(crate) fn generate(resource: &Resource) -> TokenStream {
    let ident = &resource.ident;
    let param_type_defs = resource.methods.iter().flat_map(|method| method.params.iter().filter_map(|param| param.typ.type_def()));
    let method_builders = resource.methods.iter().map(method_builder::generate);
    let nested_resource_mods = resource.resources.iter().map(generate);
    quote!{
        mod #ident {
            mod params {
                #(#param_type_defs)*
            }
            #(#method_builders)*
            #(#nested_resource_mods)*
        }
    }
}
