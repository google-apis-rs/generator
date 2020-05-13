use crate::{
    markdown, to_ident, to_rust_varstr, Method, Param, ParamInitMethod, PropertyDesc, RefOrType,
    Type, TypeDesc,
};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::BTreeMap;
use std::str::FromStr;
use syn::parse_quote;

pub(crate) fn generate(
    root_url: &str,
    service_path: &str,
    global_params: &[Param],
    method: &Method,
    creator_ident: &syn::Ident,
    schemas: &BTreeMap<syn::Ident, Type>,
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

    let param_methods = optional_params
        .iter()
        .filter(|param| {
            // We have special handling for fields and alt. Don't provide methods to set them.
            !["alt", "fields"].contains(&param.id.as_str())
        })
        .map(|param| {
            let name = &param.ident;
            let fn_def = match param.init_method() {
                ParamInitMethod::BytesInit => quote! {
                    pub fn #name(mut self, value: impl Into<Vec<u8>>) -> Self {
                        let v: Vec<u8> = value.into();
                        self.#name = Some(v.into());
                        self
                    }
                },
                ParamInitMethod::IntoImpl(param_type) => quote! {
                    pub fn #name(mut self, value: impl Into<#param_type>) -> Self {
                        self.#name = Some(value.into());
                        self
                    }
                },
                ParamInitMethod::ByValue => {
                    let param_type = param.typ.type_path();
                    quote! {
                        pub fn #name(mut self, value: #param_type) -> Self {
                            self.#name = Some(value);
                            self
                        }
                    }
                }
            };
            let description = &param
                .description
                .as_ref()
                .map(|s| markdown::sanitize(s.as_str()))
                .unwrap_or_else(String::new);
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
    let (iter_methods, iter_types_and_impls) = iter_defs(method, schemas);
    let download_method = download_method(&base_url, method);
    let upload_methods = upload_methods(root_url, method);
    let builder_doc = builder_doc(method, creator_ident);

    quote! {
        #[doc = #builder_doc]
        #[derive(Debug,Clone)]
        pub struct #builder_name<'a> {
            pub(crate) reqwest: &'a ::reqwest::Client,
            pub(crate) auth: &'a dyn ::google_api_auth::GetAccessToken,
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

        #iter_types_and_impls
    }
}

fn exec_method(
    request: Option<&RefOrType<'static>>,
    response: Option<&RefOrType<'static>>,
) -> TokenStream {
    let set_body = request.map(|_| {
        quote! {
            let req = req.json(&self.request);
        }
    });
    match response {
        Some(typ) => {
            let resp_type_path = typ.type_path();
            quote! {
                /// Execute the given operation. The fields requested are
                /// determined by the FieldSelector attribute of the return type.
                /// This allows for flexible and ergonomic partial responses. See
                /// `execute_standard` and `execute_debug` for interfaces that
                /// are not generic over the return type and deserialize the
                /// response into an auto-generated struct will all possible
                /// fields.
                pub async fn execute<T>(self) -> Result<T, crate::Error>
                where
                    T: ::serde::de::DeserializeOwned + ::google_field_selector::FieldSelector,
                {
                    let fields = ::google_field_selector::to_string::<T>();
                    let fields: Option<String> = if fields.is_empty() {
                        None
                    } else {
                        Some(fields)
                    };
                    self.execute_with_fields(fields).await
                }

                /// Execute the given operation. This will not provide any
                /// `fields` selector indicating that the server will determine
                /// the fields returned. This typically includes the most common
                /// fields, but it will not include every possible attribute of
                /// the response resource.
                pub async fn execute_with_default_fields(self) -> Result<#resp_type_path, crate::Error> {
                    self.execute_with_fields(None::<&str>).await
                }

                /// Execute the given operation. This will provide a `fields`
                /// selector of `*`. This will include every attribute of the
                /// response resource and should be limited to use during
                /// development or debugging.
                pub async fn execute_with_all_fields(self) -> Result<#resp_type_path, crate::Error> {
                    self.execute_with_fields(Some("*")).await
                }

                /// Execute the given operation. This will use the `fields`
                /// selector provided and will deserialize the response into
                /// whatever return value is provided.
                pub async fn execute_with_fields<T, F>(mut self, fields: Option<F>) -> Result<T, crate::Error>
                where
                    T: ::serde::de::DeserializeOwned,
                    F: Into<String>,
                {
                    self.fields = fields.map(Into::into);
                    self._execute().await
                }

                async fn _execute<T>(&mut self) -> Result<T, crate::Error>
                where
                    T: ::serde::de::DeserializeOwned,
                {
                    let req = self._request(&self._path())?;
                    #set_body
                    Ok(req.send().await?.error_for_status()?.json().await?)
                }
            }
        }
        None => {
            quote! {
                pub async fn execute(self) -> Result<(), crate::Error> {
                    let req = self._request(&self._path())?;
                    #set_body
                    req.send().await?.error_for_status()?;
                    Ok(())
                }
            }
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
    use std::borrow::Cow;
    let template_ast = PathTemplate::new(path_template)
        .expect(&format!("invalid path template: {}", path_template));
    let tokens = template_ast
        .nodes()
        .map(|node| match node {
            PathAstNode::Lit(lit) => {
                use ::percent_encoding::{CONTROLS, AsciiSet, utf8_percent_encode};
                const LITERALS: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'\'').add(b'<').add(b'>').add(b'\\').add(b'^').add(b'`').add(b'{').add(b'|').add(b'}');
                let escaped_lit = utf8_percent_encode(lit, LITERALS).to_string();
                quote! {output.push_str(#escaped_lit);}
            },
            PathAstNode::Var {
                var_name,
                expansion_style: expansion,
            } => {
                use crate::path_templates::ExpansionStyle;
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
                let ident = &param.ident;

                let is_array_of_strings = |type_desc: &TypeDesc| {
                    if let TypeDesc::Array{items} = type_desc {
                        if let RefOrType::Type(Cow::Owned(Type{type_desc: TypeDesc::String, ..})) = **items {
                            return true;
                        }
                    }
                    false
                };

                let var_as_str = match expansion {
                    ExpansionStyle::Simple{..} | ExpansionStyle::Reserved{..} => {
                        match &param.typ.type_desc {
                            TypeDesc::String => {
                                quote!{let var_as_str = &self.#ident;}
                            },
                            TypeDesc::Int32
                            | TypeDesc::Int64
                            | TypeDesc::Uint32
                            | TypeDesc::Uint64
                            | TypeDesc::Bytes
                            | TypeDesc::Enum { .. } => {
                                quote!{
                                    let var_as_string = self.#ident.to_string();
                                    let var_as_str = &var_as_string;
                                }
                            }
                            t => panic!(
                                "Unsupported parameter type in path: variable: {}, type: {:?}",
                                var_name, t
                            ),
                        }
                    },
                    ExpansionStyle::PathSegment => {
                        if is_array_of_strings(&param.typ.type_desc) {
                            quote!{
                                let path_iter = self.#ident.iter().map(|path_segment| {
                                    ::percent_encoding::utf8_percent_encode(path_segment, crate::SIMPLE)
                                });
                            }
                        } else {
                            panic!("PathSegment variable expansion can only be invoked on arrays of strings: {:?}", param.typ.type_desc);
                        }
                    }
                };

                let prefix_limit = match expansion {
                    ExpansionStyle::Simple{prefix: Some(prefix)} | ExpansionStyle::Reserved{prefix: Some(prefix)} => {
                        quote!{let var_as_str = &var_as_str[..#prefix as usize];}
                    },
                    ExpansionStyle::Simple{prefix: None} | ExpansionStyle::Reserved{prefix: None} => {
                        quote!{}
                    }
                    ExpansionStyle::PathSegment => {
                        quote!{}
                    }
                };

                let append_to_output = match expansion {
                    ExpansionStyle::Simple{..} => {
                        quote! {
                            output.extend(::percent_encoding::utf8_percent_encode(&var_as_str, crate::SIMPLE));
                        }
                    },
                    ExpansionStyle::Reserved{..} => {
                        quote! {
                            output.extend(::percent_encoding::utf8_percent_encode(&var_as_str, crate::RESERVED));
                        }
                    }
                    ExpansionStyle::PathSegment => {
                        quote!{
                            for segment in path_iter {
                                output.push_str("/");
                                output.extend(segment);
                            }
                        }
                    }
                };
                quote!{
                    {
                        #var_as_str
                        #prefix_limit
                        #append_to_output
                    }
                }
            }
        });
    quote! {
        fn #method_name(&self) -> String {
            let mut output = #base_url.to_owned();
            #(#tokens)*
            output
        }
    }
}

fn reqwest_http_method(http_method: &::reqwest::Method) -> syn::Path {
    match *http_method {
        ::reqwest::Method::GET => parse_quote! {::reqwest::Method::GET},
        ::reqwest::Method::POST => parse_quote! {::reqwest::Method::POST},
        ::reqwest::Method::PUT => parse_quote! {::reqwest::Method::PUT},
        ::reqwest::Method::DELETE => parse_quote! {::reqwest::Method::DELETE},
        ::reqwest::Method::HEAD => parse_quote! {::reqwest::Method::HEAD},
        ::reqwest::Method::OPTIONS => parse_quote! {::reqwest::Method::OPTIONS},
        ::reqwest::Method::CONNECT => parse_quote! {::reqwest::Method::CONNECT},
        ::reqwest::Method::PATCH => parse_quote! {::reqwest::Method::PATCH},
        ::reqwest::Method::TRACE => parse_quote! {::reqwest::Method::TRACE},
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

    let http_method = ::reqwest::Method::from_str(http_method)
        .expect(format!("unknown http method: {}", http_method).as_str());
    let reqwest_method = reqwest_http_method(&http_method);
    quote! {
        fn _request(&self, path: &str) -> Result<::reqwest::RequestBuilder, crate::Error> {
            let req = self.reqwest.request(#reqwest_method, path);
            #(let req = req.query(&[#query_params]);)*
            let req = req.bearer_auth(self.auth.access_token().map_err(|err| crate::Error::OAuth2(err))?);
            Ok(req)
        }
    }
}

fn iterable_method_impl<'a>(method: &Method) -> TokenStream {
    let builder_name = method.builder_name();
    quote! {
        impl<'a> crate::iter::IterableMethod for #builder_name<'a> {
            fn set_page_token(&mut self, value: String) {
                self.page_token = value.into();
            }

            fn execute<T>(&mut self) -> Result<T, crate::Error>
            where
                T: ::serde::de::DeserializeOwned,
            {
                // FIXME: make `IterableMethod` async/refactor `Iter` impl to async `Stream`
                self._execute()
            }
        }
    }
}

fn iter_defs(method: &Method, schemas: &BTreeMap<syn::Ident, Type>) -> (TokenStream, TokenStream) {
    use crate::PageTokenParam;
    let page_token_param = method.is_iterable(schemas);
    if page_token_param == PageTokenParam::None {
        return (quote! {}, quote! {});
    }
    let response = method.response.as_ref().unwrap(); // unwrap safe because is_iterable returned true.
    let response_type_path = response.type_path();
    let response_type_desc: &TypeDesc = &response.get_type(schemas).type_desc;
    let array_props: Vec<(&PropertyDesc, syn::TypePath)> =
        if let TypeDesc::Object { props, .. } = response_type_desc {
            props
                .values()
                .filter_map(|prop| match prop.typ.get_type(schemas).type_desc {
                    TypeDesc::Array { ref items } => Some((prop, items.type_path())),
                    _ => None,
                })
                .collect()
        } else {
            panic!("is_iterable that doesn't return an object");
        };
    let array_iter_methods = array_props.iter().map(|(prop, items_type)| {
        let prop_id = &prop.id;
        let iter_method_ident: syn::Ident =
            to_ident(&to_rust_varstr(&format!("iter_{}", &prop.ident)));
        let iter_method_ident_default: syn::Ident =
            to_ident(&to_rust_varstr(&format!("{}_with_default_fields", &iter_method_ident)));
        let iter_method_ident_all: syn::Ident =
            to_ident(&to_rust_varstr(&format!("{}_with_all_fields", &iter_method_ident)));
        let iter_method_ident_fields: syn::Ident =
            to_ident(&to_rust_varstr(&format!("{}_with_fields", &iter_method_ident)));
        quote! {
            /// Return an iterator that iterates over all `#prop_ident`. The
            /// items yielded by the iterator are chosen by the caller of this
            /// method and must implement `Deserialize` and `FieldSelector`. The
            /// populated fields in the yielded items will be determined by the
            /// `FieldSelector` implementation.
            pub fn #iter_method_ident<T>(self) -> crate::iter::PageItemIter<Self, T>
            where
                T: ::serde::de::DeserializeOwned + ::google_field_selector::FieldSelector,
            {
                let fields = ::google_field_selector::to_string::<T>();
                let fields: Option<String> = if fields.is_empty() {
                    None
                } else {
                    Some(fields)
                };
                self.#iter_method_ident_fields(fields)
            }

            /// Return an iterator that iterates over all `#prop_ident`. The
            /// items yielded by the iterator are `#items_type`. The populated
            /// fields in `#items_type` will be the default fields populated by
            /// the server.
            pub fn #iter_method_ident_default(self) -> crate::iter::PageItemIter<Self, #items_type> {
                self.#iter_method_ident_fields(None::<String>)
            }

            /// Return an iterator that iterates over all `#prop_ident`. The
            /// items yielded by the iterator are `#items_type`. The populated
            /// fields in `#items_type` will be all fields available. This should
            /// primarily be used during developement and debugging as fetching
            /// all fields can be expensive both in bandwidth and server
            /// resources.
            pub fn #iter_method_ident_all(self) -> crate::iter::PageItemIter<Self, #items_type> {
                self.#iter_method_ident_fields(Some("*"))
            }

            pub fn #iter_method_ident_fields<T, F>(mut self, fields: Option<F>) -> crate::iter::PageItemIter<Self, T>
            where
                T: ::serde::de::DeserializeOwned,
                F: AsRef<str>,
            {
                self.fields = Some({
                    let mut selector = concat!("nextPageToken,", #prop_id).to_owned();
                    let items_fields = fields.as_ref().map(|x| x.as_ref()).unwrap_or("");
                    if !items_fields.is_empty() {
                        selector.push_str("(");
                        selector.push_str(items_fields);
                        selector.push_str(")");
                    }
                    selector
                });
                crate::iter::PageItemIter::new(self, #prop_id)
            }
        }
    });

    let iter_methods = quote! {
        #(#array_iter_methods)*

        pub fn iter<T>(self) -> crate::iter::PageIter<Self, T>
        where
            T: ::serde::de::DeserializeOwned + ::google_field_selector::FieldSelector,
        {
            let fields = ::google_field_selector::to_string::<T>();
            let fields: Option<String> = if fields.is_empty() {
                None
            } else {
                Some(fields)
            };
            self.iter_with_fields(fields)
        }

        pub fn iter_with_default_fields(self) -> crate::iter::PageIter<Self, #response_type_path> {
            self.iter_with_fields(None::<&str>)
        }

        pub fn iter_with_all_fields(self) -> crate::iter::PageIter<Self, #response_type_path> {
            self.iter_with_fields(Some("*"))
        }

        pub fn iter_with_fields<T, F>(mut self, fields: Option<F>) -> crate::iter::PageIter<Self, T>
        where
            T: ::serde::de::DeserializeOwned,
            F: AsRef<str>,
        {
            let mut fields = fields.as_ref().map(|x| x.as_ref()).unwrap_or("").to_owned();
            if !fields.is_empty() {
                // Append nextPageToken to any non-empty field selector.
                // Requesting the same field twice is not harmful, so this will
                // work even if the FieldSelector includes it. We do not do this
                // if fields is empty because an empty field selector is
                // requesting the default set of fields, and specifying
                // nextPageToken would only request that one field. The default
                // set of fields always seems to include nextPageToken anyway.
                match fields.chars().rev().nth(0) {
                    Some(',') | None => {},
                    _ => fields.push_str(","),
                }
                fields.push_str("nextPageToken");
                self.fields = Some(fields);
            }
            crate::iter::PageIter::new(self)
        }
    };

    let iterable_method_impl = iterable_method_impl(method);
    (iter_methods, iterable_method_impl)
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
        pub async fn download<W>(mut self, output: &mut W) -> Result<u64, crate::Error>
        where
            W: futures::io::AsyncWriteExt + std::marker::Unpin + ?Sized,
        {
            self.alt = Some(crate::params::Alt::Media);
            let response = self._request(&self._path())?.send().await?.error_for_status()?;
            Ok(futures::io::copy(response.bytes_stream(), output).await?)
        }
    }
}

fn upload_methods(base_url: &str, method: &Method) -> TokenStream {
    if let Some(media_upload) = &method.media_upload {
        let simple_fns = media_upload.simple_path.as_ref().map(|path| {
            let path_fn = path_method(&parse_quote!{_simple_upload_path}, base_url, path, &method.params);
            let add_request_part = method.request.as_ref().map(|_| {
                quote!{
                    let request_json = ::serde_json::to_vec(&self.request)?;
                    multipart.new_part(Part::new(::mime::APPLICATION_JSON, Box::new(::std::io::Cursor::new(request_json))));
                }
            });
            let upload_fn = match &method.response {
                Some(_response) => {
                    quote!{
                        pub async fn upload<T, R>(mut self, content: R, mime_type: ::mime::Mime) -> Result<T, crate::Error>
                        where
                            T: ::serde::de::DeserializeOwned + ::google_field_selector::FieldSelector,
                            R: ::std::io::Read + ::std::io::Seek + Send + 'static,
                        {
                            let fields = ::google_field_selector::to_string::<T>();
                            self.fields = if fields.is_empty() {
                                None
                            } else {
                                Some(fields)
                            };
                            let req = self._request(&self._simple_upload_path())?;
                            let req = req.query(&[("uploadType", "multipart")]);
                            use crate::multipart::{RelatedMultiPart, Part};
                            let mut multipart = RelatedMultiPart::new();
                            #add_request_part
                            multipart.new_part(Part::new(mime_type, Box::new(content)));
                            let req = req.header(::reqwest::header::CONTENT_TYPE, format!("multipart/related; boundary={}", multipart.boundary()));
                            let req = req.body(reqwest::Body::new(multipart.into_reader()));
                            let response = req.send().await?.error_for_status()?;
                            Ok(response)
                        }
                    }
                },
                None => {
                    quote!{
                        pub fn upload<R>(self, content: R, mime_type: ::mime::Mime) -> Result<(), crate::Error>
                        where
                            R: ::std::io::Read + ::std::io::Seek + Send + 'static,
                        {
                            let req = self._request(&self._simple_upload_path())?;
                            let req = req.query(&[("uploadType", "multipart")]);
                            use crate::multipart::{RelatedMultiPart, Part};
                            let mut multipart = RelatedMultiPart::new();
                            #add_request_part
                            multipart.new_part(Part::new(mime_type, Box::new(content)));
                            let req = req.header(::reqwest::header::CONTENT_TYPE, format!("multipart/related; boundary={}", multipart.boundary()));
                            let req = req.body(reqwest::Body::new(multipart.into_reader()));
                            req.send().await?.error_for_status()?;
                            Ok(())
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
                pub async fn start_resumable_upload(self, mime_type: ::mime::Mime) -> Result<crate::ResumableUpload, crate::Error> {
                    let req = self._request(&self._resumable_upload_path())?;
                    let req = req.query(&[("uploadType", "resumable")]);
                    let req = req.header(::reqwest::header::HeaderName::from_static("x-upload-content-type"), mime_type.to_string());
                    #set_body
                    let resp = req.send().await?.error_for_status()?;
                    let location_header = resp.headers().get(::reqwest::header::LOCATION).ok_or_else(|| crate::Error::Other(format!("No LOCATION header returned when initiating resumable upload").into()))?;
                    let upload_url = ::std::str::from_utf8(location_header.as_bytes()).map_err(|_| crate::Error::Other(format!("Non UTF8 LOCATION header returned").into()))?.to_owned();
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

fn builder_doc(method: &Method, creator_ident: &syn::Ident) -> String {
    format!(
        "Created via [{}::{}()](struct.{}.html#method.{})",
        creator_ident, &method.ident, creator_ident, &method.ident
    )
}
