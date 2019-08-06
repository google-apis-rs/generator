use crate::{to_ident, to_rust_typestr, to_rust_varstr, Method, TypeDesc, Param};
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;


pub(crate) fn generate(method: &Method) -> TokenStream {
    let builder_name = method.builder_name();
    let (required_params, optional_params): (Vec<_>, _) =
        method.params.iter().partition(|param| param.required);

    let mut builder_fields: Vec<syn::Field> = required_params
        .iter()
        .map(|&param| {
            let ident = &param.ident;
            let ty: syn::Type = param.typ.type_path().into();
            use syn::parse::Parser;
            syn::Field::parse_named
                .parse2(quote! {
                    #ident: #ty
                })
                .expect("failed to parse param field")
        })
        .collect();
    builder_fields.extend(optional_params.iter().map(|&param| {
        let ident = &param.ident;
        let ty = param.typ.type_path();
        use syn::parse::Parser;
        syn::Field::parse_named
            .parse2(quote! {
                #ident: Option<#ty>
            })
            .expect("failed to parse param field")
    }));

    let param_methods = optional_params.iter().map(|param| {
        let fn_name = to_ident(&to_rust_varstr(&format!("{}", param.ident)));
        let fn_def = match param.typ.type_desc {
            TypeDesc::String => {
                param_into_method(&fn_name, &param.ident, parse_quote! {impl Into<String>})
            }
            TypeDesc::Array { ref items } => {
                let items_type = items.type_path();
                param_into_method(
                    &fn_name,
                    &param.ident,
                    parse_quote! {impl Into<Box<[#items_type]>>},
                )
            }
            _ => param_value_method(&fn_name, &param.ident, param.typ.type_path().into()),
        };
        let description = &param.description;
        quote! {
            #[doc = #description]
            #fn_def
        }
    });

    let path_method = path_method(&method.path, &method.params);

    quote! {
        #[derive(Debug,Clone)]
        pub struct #builder_name {
            #(#builder_fields,)*

        }

        impl #builder_name {
            #(#param_methods)*
        }
    }
}

fn param_into_method(
    fn_name: &syn::Ident,
    param_ident: &syn::Ident,
    param_type: syn::Type,
) -> TokenStream {
    quote! {
        pub fn #fn_name(mut self, value: #param_type) -> Self {
            self.#param_ident = Some(value.into());
            self
        }
    }
}

fn param_value_method(
    fn_name: &syn::Ident,
    param_ident: &syn::Ident,
    param_type: syn::Type,
) -> TokenStream {
    quote! {
        pub fn #fn_name(mut self, value: #param_type) -> Self {
            self.#param_ident = Some(value);
            self
        }
    }
}

fn path_method(
    path_template: &str,
    params: &[Param],
) -> TokenStream {
    use crate::path_templates::{PathTemplate, PathAstNode};
    let template_ast = PathTemplate::new(path_template).expect(format!("invalid path template: {}", path_template));
    let tokens: Vec<TokenStream> = template_ast.nodes().map(|node| {
        match node {
            PathAstNode::Lit(lit) => quote!{output.push_str(#lit)},
            PathAstNode::Var{var_name, expansion_style} => quote!{ output.push_var(#var_name) },
        }
    }).collect();
    quote!{
        fn _path(&self) -> String {
            let mut output = String::new();
            #(#tokens)*
        }
    }
}
