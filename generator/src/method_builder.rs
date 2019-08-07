use crate::{
    to_ident, to_rust_typestr, to_rust_varstr, Method, Param, PropertyDesc, Type, TypeDesc,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;

pub(crate) fn generate(base_url: &str, global_params: &[Param], method: &Method) -> TokenStream {
    if is_iter_method(method) {
        println!("method: {} is iterable", method.id);
    }
    let builder_name = method.builder_name();
    let all_params = global_params.into_iter().chain(method.params.iter());
    let (required_params, optional_params): (Vec<_>, _) =
        all_params.clone().partition(|param| param.required);

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

    let path_method = path_method(base_url, &method.path, &method.params);
    let request_method = request_method(&method.http_method, all_params);
    let exec_method = exec_method(method.response.as_ref());

    quote! {
        #[derive(Debug,Clone)]
        pub struct #builder_name<'a> {
            pub(super) reqwest: &'a ::reqwest::Client,
            #(#builder_fields,)*
        }

        impl<'a> #builder_name<'a> {
            #(#param_methods)*

            #exec_method

            #path_method
            #request_method

        }
    }
}

fn exec_method(response: Option<&Type>) -> TokenStream {
    match response {
        Some(typ) => {
            let resp_type_path = typ.type_path();
            quote! {
                pub fn execute<T: ::serde::de::DeserializeOwned + ::field_selector::FieldSelector>(mut self) -> Result<T, Box<dyn ::std::error::Error>> {
                    if self.fields.is_none() {
                        self.fields = Some(T::field_selector());
                    }
                    Ok(self._request().send()?.error_for_status()?.json()?)
                }

                pub fn execute_text(self) -> Result<String, Box<dyn ::std::error::Error>> {
                    Ok(self._request().send()?.error_for_status()?.text()?)
                }

                pub fn execute_debug(self) -> Result<#resp_type_path, Box<dyn ::std::error::Error>> {
                    self.execute()
                }
            }
        }
        None => {
            quote! {
                pub fn execute(self) -> Result<(), Box<dyn ::std::error::Error>> {
                    self._request().send()?.error_for_status()?;
                    Ok(())
                }
            }
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

fn path_method(base_url: &str, path_template: &str, params: &[Param]) -> TokenStream {
    use crate::path_templates::{PathAstNode, PathTemplate};
    let template_ast = PathTemplate::new(path_template)
        .expect(&format!("invalid path template: {}", path_template));
    let tokens: Vec<TokenStream> = template_ast
        .nodes()
        .map(|node| match node {
            PathAstNode::Lit(lit) => quote! {output.push_str(#lit);},
            PathAstNode::Var {
                var_name,
                expansion_style,
            } => {
                let param = params
                    .iter()
                    .find(|p| &p.id == var_name)
                    .expect(&format!("failed to find var {}", var_name));
                if !param.required {
                    panic!(
                        "path template {} uses param {}, which is not required",
                        path_template, &param.id
                    );
                }
                match &param.typ.type_desc {
                    TypeDesc::String => {
                        let ident = &param.ident;
                        quote! { output.push_str(&self.#ident); }
                    }
                    TypeDesc::Int32
                    | TypeDesc::Int64
                    | TypeDesc::Uint32
                    | TypeDesc::Uint64
                    | TypeDesc::Enum { .. } => {
                        let ident = &param.ident;
                        quote! {
                            {
                                let str_value = self.#ident.to_string();
                                output.push_str(&str_value);
                            }
                        }
                    }
                    t => panic!(
                        "Unsupported parameter type in path: variable: {}, type: {:?}",
                        var_name, t
                    ),
                }
            }
        })
        .collect();
    quote! {
        fn _path(&self) -> String {
            let mut output = #base_url.to_owned();
            #(#tokens)*
            output
        }
    }
}

fn reqwest_http_method(http_method: &str) -> syn::Path {
    match http_method {
        "GET" => parse_quote! {::reqwest::Method::GET},
        "POST" => parse_quote! {::reqwest::Method::POST},
        "PUT" => parse_quote! {::reqwest::Method::PUT},
        "DELETE" => parse_quote! {::reqwest::Method::DELETE},
        "HEAD" => parse_quote! {::reqwest::Method::HEAD},
        "OPTIONS" => parse_quote! {::reqwest::Method::OPTIONS},
        "CONNECT" => parse_quote! {::reqwest::Method::CONNECT},
        "PATCH" => parse_quote! {::reqwest::Method::PATCH},
        "TRACE" => parse_quote! {::reqwest::Method::TRACE},
        _ => panic!("unknown http method: {}", http_method),
    }
}

fn request_method<'a>(http_method: &str, params: impl Iterator<Item = &'a Param>) -> TokenStream {
    let method = reqwest_http_method(http_method);
    let query_params = params
        .filter(|param| param.location == "query")
        .map(|param| {
            let id = &param.id;
            let ident = &param.ident;
            quote! {(#id, &self.#ident)}
        });

    quote! {
        pub fn _request(&self) -> ::reqwest::RequestBuilder {
            let req = self.reqwest.request(#method, &self._path());
            #(let req = req.query(&[#query_params]);)*
            // Hack until real oauth token support is implemented.
            let req = req.bearer_auth(crate::auth_token());
            req
        }
    }
}

fn is_iter_method(method: &Method) -> bool {
    // The requirements to qualify as an iterator are
    // The method needs to define a response object.
    // The response object needs to have a nextPageToken.
    // There needs to be a pageToken query param.
    let response_contains_next_page_token = method
        .response
        .as_ref()
        .map(|resp_type| {
            if let TypeDesc::Object { props, .. } = &resp_type.type_desc {
                props
                    .values()
                    .find(|PropertyDesc { id, typ, .. }| {
                        if let TypeDesc::String = typ.type_desc {
                            if id == "nextPageToken" {
                                return true;
                            }
                        }
                        false
                    })
                    .is_some()
            } else {
                false
            }
        })
        .unwrap_or(false);
    let params_contains_page_token = method
        .params
        .iter()
        .find(|param| {
            if let TypeDesc::String = param.typ.type_desc {
                if param.id == "pageToken" {
                    return true;
                }
            }
            false
        })
        .is_some();
    response_contains_next_page_token && params_contains_page_token
}
