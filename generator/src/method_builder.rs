use crate::{to_ident, to_rust_typestr, to_rust_varstr, Method};
use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn generate(method: &Method) -> TokenStream {
    let builder_name = to_ident(&to_rust_typestr(&format!("{}-Call", &method.id)));
    let optional_params = method.params.iter().filter(|param| !param.required);

    let param_methods = optional_params.map(|param| {
        let fn_name = to_ident(&to_rust_varstr(&format!("{}", param.ident)));
        let param_type = &param.typ.type_path();
        quote! {
            fn #fn_name(self, value: #param_type) -> Self {
                self
            }
        }
    });

    quote! {
        struct #builder_name {

        }

        impl #builder_name {
            #(#param_methods)*
        }
    }
}