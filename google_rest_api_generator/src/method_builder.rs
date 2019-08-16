use crate::{to_ident, to_rust_varstr, Method, Param, PropertyDesc, Type, TypeDesc};
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;

pub(crate) fn generate(
    root_url: &str,
    service_path: &str,
    global_params: &[Param],
    method: &Method,
) -> TokenStream {
    let builder_name = method.builder_name();
    let all_params = method.params.iter().chain(global_params.into_iter());
    let (required_params, optional_params): (Vec<_>, _) =
        all_params.clone().partition(|param| param.required);

    let mut builder_fields: Vec<syn::Field> = Vec::new();
    if let Some(req) = method.request.as_ref() {
        let ty: syn::Type = req.type_path().into();
        use syn::parse::Parser;
        builder_fields.push(
            syn::Field::parse_named
                .parse2(parse_quote! {request: #ty})
                .expect("failed to parse request field"),
        );
    }
    builder_fields.extend(required_params.iter().map(|&param| {
        let ident = &param.ident;
        let ty: syn::Type = param.typ.type_path().into();
        use syn::parse::Parser;
        syn::Field::parse_named
            .parse2(quote! {
                #ident: #ty
            })
            .expect("failed to parse param field")
    }));
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
        let description = &param.description.as_ref().map(|s| s.as_str()).unwrap_or("");
        quote! {
            #[doc = #description]
            #fn_def
        }
    });

    let base_url = format!("{}{}", root_url, service_path);
    let default_path_method = path_method(
        &parse_quote! {_path},
        &base_url,
        &method.path,
        &method.params,
    );
    let request_method = request_method(&method.http_method, all_params);
    let exec_method = exec_method(method.request.as_ref(), method.response.as_ref());
    let iterable_method_impl = iterable_method_impl(method);
    let iter_methods = iter_methods(method);
    let download_method = download_method(&base_url, method);
    let upload_methods = upload_methods(root_url, method);

    quote! {
        #[derive(Debug,Clone)]
        pub struct #builder_name<'a> {
            pub(crate) reqwest: &'a ::reqwest::Client,
            #(#builder_fields,)*
        }

        impl<'a> #builder_name<'a> {
            #(#param_methods)*

            #iter_methods
            #download_method
            #upload_methods
            #exec_method

            #default_path_method
            #request_method
        }

        #iterable_method_impl
    }
}

fn exec_method(request: Option<&Type>, response: Option<&Type>) -> TokenStream {
    let set_body = request.map(|_| {
        quote! {
            let req = req.json(&self.request);
        }
    });
    match response {
        Some(typ) => {
            let resp_type_path = typ.type_path();
            quote! {
                pub fn execute<T>(mut self) -> Result<T, Box<dyn ::std::error::Error>>
                where
                    T: ::serde::de::DeserializeOwned + ::field_selector::FieldSelector,
                {
                    self._execute()
                }

                /// TODO: Remove once development debugging is no longer a priority.
                pub fn execute_text(self) -> Result<String, Box<dyn ::std::error::Error>> {
                    let req = self._request(&self._path());
                    #set_body
                    Ok(req.send()?.error_for_status()?.text()?)
                }

                pub fn execute_debug(self) -> Result<#resp_type_path, Box<dyn ::std::error::Error>> {
                    self.execute()
                }

                fn _execute<T>(&mut self) -> Result<T, Box<dyn ::std::error::Error>>
                where
                    T: ::serde::de::DeserializeOwned + ::field_selector::FieldSelector,
                {
                    if self.fields.is_none() {
                        self.fields = Some(T::field_selector());
                    }
                    let req = self._request(&self._path());
                    #set_body
                    Ok(req.send()?.error_for_status()?.json()?)
                }
            }
        }
        None => {
            quote! {
                pub fn execute(self) -> Result<(), Box<dyn ::std::error::Error>> {
                    let req = self._request(&self._path());
                    #set_body
                    req.send()?.error_for_status()?;
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
        pub fn #fn_name(&mut self, value: #param_type) -> &mut Self {
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
        pub fn #fn_name(&mut self, value: #param_type) -> &mut Self {
            self.#param_ident = Some(value);
            self
        }
    }
}

fn path_method(
    method_name: &syn::Ident,
    base_url: &str,
    path_template: &str,
    params: &[Param],
) -> TokenStream {
    use crate::path_templates::{PathAstNode, PathTemplate};
    let template_ast = PathTemplate::new(path_template)
        .expect(&format!("invalid path template: {}", path_template));
    let tokens: Vec<TokenStream> = template_ast
        .nodes()
        .map(|node| match node {
            PathAstNode::Lit(lit) => quote! {output.push_str(#lit);},
            PathAstNode::Var {
                var_name,
                expansion_style: _,
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
        fn #method_name(&self) -> String {
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
    let query_params = params
        .filter(|param| param.location == "query")
        .map(|param| {
            let id = &param.id;
            let ident = &param.ident;
            quote! {(#id, &self.#ident)}
        });

    /*
        let set_body = request_type.map(|_| {
            quote! {
                let req = req.json(&self.request);
            }
        });
    */
    let reqwest_method = reqwest_http_method(http_method);
    quote! {
        fn _request(&self, path: &str) -> ::reqwest::RequestBuilder {
            let req = self.reqwest.request(#reqwest_method, path);
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

fn iterable_method_impl(method: &Method) -> TokenStream {
    if !is_iter_method(method) {
        return quote! {};
    }
    let builder_name = method.builder_name();
    quote! {
        impl<'a> crate::IterableMethod for #builder_name<'a> {
            fn set_page_token(&mut self, value: String) {
                self.page_token = value.into();
            }

            fn execute<T>(&mut self) -> Result<T, Box<dyn ::std::error::Error>>
            where
                T: ::serde::de::DeserializeOwned + ::field_selector::FieldSelector,
            {
                self._execute()
            }
        }
    }
}

fn iter_methods(method: &Method) -> TokenStream {
    if !is_iter_method(method) {
        return quote! {};
    }

    let array_props: Vec<&PropertyDesc> = if let Some(Type {
        type_desc: TypeDesc::Object { props, .. },
        ..
    }) = &method.response
    {
        props
            .values()
            .filter(|prop| match prop.typ.type_desc {
                TypeDesc::Array { .. } => true,
                _ => false,
            })
            .collect()
    } else {
        Vec::new()
    };
    let array_iter_methods = array_props.iter().map(|prop| {
        let iter_method_ident: syn::Ident = syn::parse_str(&format!("iter_{}", &prop.ident)).unwrap();
        let prop_id = &prop.id;
        quote!{
            pub fn #iter_method_ident<T>(&'a mut self) -> impl Iterator<Item=Result<T, Box<dyn ::std::error::Error>>> + 'a
            where
                T: ::serde::de::DeserializeOwned + ::field_selector::FieldSelector + 'a,
            {

                struct ItemIter<'a, M, T>{
                    method: &'a mut M,
                    finished: bool,
                    items_iter: Option<::std::vec::IntoIter<T>>,
                }

                impl<'a, M, T> Iterator for ItemIter<'a, M, T>
                where
                    M: crate::IterableMethod,
                    T: ::serde::de::DeserializeOwned + ::field_selector::FieldSelector,
                {
                    type Item = Result<T, Box<dyn ::std::error::Error>>;

                    fn next(&mut self) -> Option<Result<T, Box<dyn ::std::error::Error>>> {
                        use ::field_selector::FieldSelector;
                        #[derive(::serde::Deserialize,FieldSelector)]
                        struct Resp<T>
                        where
                            T: FieldSelector,
                        {
                            #[serde(rename=#prop_id)]
                            items: Option<Vec<T>>,

                            #[serde(rename="nextPageToken")]
                            next_page_token: Option<String>,
                        }
                        loop {
                            if let Some(iter) = self.items_iter.as_mut() {
                                match iter.next() {
                                    Some(v) => return Some(Ok(v)),
                                    None => {},
                                }
                            }

                            if self.finished {
                                return None;
                            }

                            let resp: Resp<T> = match self.method.execute() {
                                Ok(r) => r,
                                Err(err) => return Some(Err(err)),
                            };

                            if let Some(next_page_token) = resp.next_page_token {
                                self.method.set_page_token(next_page_token);
                            } else {
                                self.finished = true;
                            }

                            self.items_iter = resp.items.map(|i| i.into_iter());
                        }
                    }
                }

                ItemIter{method: self, finished: false, items_iter: None}
            }
        }
    });

    quote! {
        #(#array_iter_methods)*
        pub fn iter<T>(&'a mut self) -> impl Iterator<Item=Result<T, Box<dyn ::std::error::Error>>> + 'a
        where
            T: ::serde::de::DeserializeOwned + ::field_selector::FieldSelector + 'a,
        {
            crate::PageIter{method: self, finished: false, _phantom: ::std::default::Default::default()}
        }
    }
}

fn download_method(base_url: &str, method: &Method) -> TokenStream {
    if !method.supports_media_download {
        return quote! {};
    }
    let download_path_method = path_method(
        &parse_quote! {_download_path},
        &format!("{}download/", base_url),
        &method.path,
        &method.params,
    );
    quote! {
        #download_path_method
        pub fn download<W>(mut self, output: &mut W) -> Result<u64, Box<dyn ::std::error::Error>>
        where
            W: ::std::io::Write + ?Sized,
        {
            self.alt(crate::params::Alt::Media);
            Ok(self._request(&self._path()).send()?.error_for_status()?.copy_to(output)?)
        }
    }
}

fn upload_methods(base_url: &str, method: &Method) -> TokenStream {
    if let Some(media_upload) = &method.media_upload {
        let simple_fns = media_upload.simple_path.as_ref().map(|path| {
            let path_fn = path_method(&parse_quote!{_simple_upload_path}, base_url, path, &method.params);
            let upload_fn = match &method.response {
                Some(_response) => {
                    quote!{
                        pub fn upload<T, R>(mut self, content: R, mime_type: ::mime::Mime) -> Result<T, Box<dyn ::std::error::Error>>
                        where
                            T: ::serde::de::DeserializeOwned + ::field_selector::FieldSelector,
                            R: ::std::io::Read + ::std::io::Seek + Send + 'static,
                        {
                            if self.fields.is_none() {
                                self.fields = Some(T::field_selector());
                            }
                            let req = self._request(&self._simple_upload_path());
                            let req = req.query(&[("uploadType", "multipart")]);
                            use crate::multipart::{RelatedMultiPart, Part};
                            // TODO: Do not read the contents of the file into memory.
                            let mut multipart = RelatedMultiPart::new();
                            let request_json = ::serde_json::to_vec(&self.request)?;
                            multipart.new_part(Part::new(::mime::APPLICATION_JSON, Box::new(::std::io::Cursor::new(request_json))));
                            multipart.new_part(Part::new(mime_type, Box::new(content)));
                            let req = req.header(::reqwest::header::CONTENT_TYPE, format!("multipart/related; boundary={}", multipart.boundary()));
                            let req = req.body(reqwest::Body::new(multipart.into_reader()));
                            Ok(req.send()?.error_for_status()?.json()?)
                        }
                    }
                },
                None => {
                    quote!{
                        pub fn upload<R>(mut self, content: R, mime_type: ::mime::Mime) -> Result<(), Box<dyn ::std::error::Error>>
                        where
                            R: ::std::io::Read + ::std::io::Seek,
                        {
                            let req = self._request(&self._simple_upload_path());
                            let req = req.query(&[("uploadType", "multipart")]);
                            Ok(req.send()?.error_for_status()?)
                        }
                    }
                }
            };
            quote!{
                #path_fn
                #upload_fn
            }
        });

        let resumable_fns = media_upload.resumable_path.as_ref().map(|path| {
            let set_body = method.request.as_ref().map(|_| {
                quote! {
                    let req = req.json(&self.request);
                }
            });

            let path_fn = path_method(
                &parse_quote! {_resumable_upload_path},
                base_url,
                path,
                &method.params,
            );
            let upload_fn = quote!{
                pub fn start_resumable_upload(self, mime_type: ::mime::Mime) -> Result<crate::ResumableUpload, Box<dyn ::std::error::Error>> {
                    let req = self._request(&self._resumable_upload_path());
                    let req = req.query(&[("uploadType", "resumable")]);
                    let req = req.header(::reqwest::header::HeaderName::from_static("x-upload-content-type"), mime_type.to_string());
                    #set_body
                    let resp = req.send()?.error_for_status()?;
                    let location_header = resp.headers().get(::reqwest::header::LOCATION).ok_or_else(|| format!("No LOCATION header returned when initiating resumable upload"))?;
                    let upload_url = ::std::str::from_utf8(location_header.as_bytes())?.to_owned();
                    Ok(crate::ResumableUpload::new(self.reqwest.clone(), upload_url))
                }
            };
            quote! {
                #path_fn
                #upload_fn
            }
        });

        quote! {
            #simple_fns
            #resumable_fns
        }
    } else {
        quote! {}
    }
}
