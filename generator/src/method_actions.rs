use crate::{to_ident, to_rust_varstr, Param, ParamInitMethod, Method};
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;

pub(crate) fn generate(method: &Method, global_params: &[Param]) -> TokenStream {
    let method_ident = to_ident(&to_rust_varstr(&method.id));
    let method_builder_type = method.builder_name();
    let mut required_args: Vec<syn::FnArg> = Vec::new();
    let mut method_builder_initializers: Vec<syn::FieldValue> = Vec::new();
    if let Some(req) = method.request.as_ref() {
        let ty = req.type_path();
        required_args.push(parse_quote! {request: #ty});
        method_builder_initializers.push(parse_quote! {request});
    }
    required_args.extend(method.params.iter().filter(|p| p.required).map(|param| {
        let name = &param.ident;
        let init_method: syn::FnArg = match param.init_method() {
            ParamInitMethod::IntoImpl(into_typ) => parse_quote! {#name: impl Into<#into_typ>},
            ParamInitMethod::ByValue => {
                let ty = param.typ.type_path();
                parse_quote! {#name: #ty}
            }
        };
        init_method
    }));
    let all_params = global_params.into_iter().chain(method.params.iter());
    method_builder_initializers.extend(all_params.map(|param| {
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
    }));
    let method_description = &method.description.as_ref().map(|s| s.as_str()).unwrap_or("");
    quote! {
        #[doc = #method_description]
        pub fn #method_ident(&self#(, #required_args)*) -> #method_builder_type {
            #method_builder_type{
                reqwest: &self.reqwest,
                #(#method_builder_initializers,)*
            }
        }
    }
}