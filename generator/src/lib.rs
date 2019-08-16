#![recursion_limit = "256"] // for quote macro

use discovery_parser::{DiscoveryRestDesc, RefOrType};
use log::info;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::collections::BTreeMap;
use std::error::Error;
use syn::parse_quote;

mod cargo;
mod method_builder;
mod path_templates;
mod resource_builder;
mod rustfmt;

pub fn generate<U, P>(discovery_url: U, base_dir: P, auth_token: &str) -> Result<(), Box<dyn Error>>
where
    U: reqwest::IntoUrl,
    P: AsRef<std::path::Path>,
{
    use std::io::Write;
    info!("getting discovery doc");
    let desc: DiscoveryRestDesc = reqwest::get(discovery_url)?.json()?;
    info!("buidling api desc");
    let api_desc = APIDesc::from_discovery(&desc);
    info!("creating directory and Cargo.toml");
    let project_path = base_dir.as_ref().join("foo");
    let src_path = project_path.join("src");
    std::fs::create_dir_all(&src_path)?;
    let cargo_path = project_path.join("Cargo.toml");
    let cargo_contents =
        cargo::cargo_toml(format!("google_{}_{}", &desc.name, &desc.version)).to_string();
    std::fs::write(&cargo_path, &cargo_contents)?;
    info!("writing lib");
    let output_file = std::fs::File::create(&src_path.join("lib.rs"))?;
    let mut rustfmt_writer = crate::rustfmt::RustFmtWriter::new(output_file)?;
    rustfmt_writer.write_all(api_desc.generate(auth_token).to_string().as_bytes())?;
    rustfmt_writer.write_all(include_bytes!("../gen_include/multipart.rs"))?;
    rustfmt_writer.write_all(include_bytes!("../gen_include/resumable_upload.rs"))?;
    rustfmt_writer.close()?;
    info!("returning");
    Ok(())
}

// A structure that represents the desired rust API. Typically built by
// transforming a discovery_parser::DiscoveryRestDesc.
#[derive(Clone, Debug)]
struct APIDesc {
    name: String,
    version: String,
    root_url: String,
    service_path: String,
    schema_types: Vec<Type>,
    params: Vec<Param>,
    resources: Vec<Resource>,
}
impl APIDesc {
    fn from_discovery(discovery_desc: &DiscoveryRestDesc) -> APIDesc {
        let mut schema_types: Vec<Type> = discovery_desc
            .schemas
            .iter()
            .map(|(_id, schema)| Type::from_disco_schema(schema, &discovery_desc.schemas))
            .collect();
        let mut params: Vec<Param> = discovery_desc
            .parameters
            .iter()
            .map(|(param_id, param_desc)| {
                Param::from_disco_param(param_id, &parse_quote! {crate::params}, param_desc)
            })
            .collect();
        let mut resources: Vec<Resource> = discovery_desc
            .resources
            .iter()
            .map(|(resource_id, resource_desc)| {
                Resource::from_disco_resource(
                    resource_id,
                    &parse_quote! {crate},
                    resource_desc,
                    &discovery_desc.schemas,
                )
            })
            .collect();
        if any_method_supports_media(&resources) {
            add_media_to_alt_param(&mut params);
        }
        schema_types.sort_by(|a, b| a.type_path_str().cmp(&b.type_path_str()));
        params.sort_by(|a, b| a.ident.cmp(&b.ident));
        resources.sort_by(|a, b| a.ident.cmp(&b.ident));
        APIDesc {
            name: discovery_desc.name.clone(),
            version: discovery_desc.version.clone(),
            root_url: discovery_desc.root_url.clone(),
            service_path: discovery_desc.service_path.clone(),
            schema_types,
            params,
            resources,
        }
    }

    fn generate(&self, auth_token: &str) -> TokenStream {
        info!("getting all types");
        let all_types = self.all_types();
        let schemas_to_create = all_types
            .iter()
            .filter(|typ| typ.parent_path == parse_quote! {crate::schemas})
            .filter_map(|typ| typ.type_def());
        let params_to_create = all_types
            .iter()
            .filter(|typ| typ.parent_path == parse_quote! {crate::params})
            .filter_map(|typ| typ.type_def());
        info!("generating resources");
        let resource_modules = self.resources.iter().map(|resource| {
            resource_builder::generate(&self.root_url, &self.service_path, &self.params, resource)
        });
        info!("creating resource actions");
        let resource_actions = self.resources.iter().map(|resource| {
            let resource_ident = &resource.ident;
            let action_ident = resource.action_type_name();
            let description = format!(
                "Actions that can be performed on the {} resource",
                &resource.ident
            );
            quote! {
                #[doc= #description]
                pub fn #resource_ident(&self) -> crate::#resource_ident::#action_ident {
                    crate::#resource_ident::#action_ident{
                        reqwest: &self.reqwest,
                    }
                }
            }
        });
        info!("outputting");
        quote! {
            // A serde helper module that can be used with the `with` attribute
            // to deserialize any string to a FromStr type and serialize any
            // Display type to a String. Google API's encode i64, u64 values as
            // strings.
            mod parsed_string {
                pub fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
                where
                    T: ::std::fmt::Display,
                    S: ::serde::Serializer,
                {
                    use ::serde::Serialize;
                    value.as_ref().map(|x| x.to_string()).serialize(serializer)
                }

                pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
                where
                    T: ::std::str::FromStr,
                    T::Err: ::std::fmt::Display,
                    D: ::serde::de::Deserializer<'de>,
                {
                    use ::serde::Deserialize;
                    match Option::<String>::deserialize(deserializer)? {
                        Some(x) => Ok(Some(x.parse().map_err(::serde::de::Error::custom)?)),
                        None => Ok(None),
                    }
                }
            }

            trait IterableMethod: Clone {
                fn set_page_token(&mut self, value: String);
                fn execute<T>(&mut self) -> Result<T, Box<dyn ::std::error::Error>>
                where
                    T: ::serde::de::DeserializeOwned + ::field_selector::FieldSelector;
            }

            struct PageIter<'a, M, T>{
                method: &'a mut M,
                finished: bool,
                _phantom: ::std::marker::PhantomData<T>,
            }

            impl<'a, M, T> Iterator for PageIter<'a, M, T>
            where
                M: IterableMethod,
                T: ::serde::de::DeserializeOwned + ::field_selector::FieldSelector,
            {
                type Item = Result<T, Box<dyn ::std::error::Error>>;

                fn next(&mut self) -> Option<Result<T, Box<dyn ::std::error::Error>>> {
                    use ::field_selector::FieldSelector;
                    #[derive(::serde::Deserialize, FieldSelector)]
                    struct PaginatedResult<T>
                    where
                        T: FieldSelector,
                    {
                        #[serde(rename="nextPageToken")]
                        next_page_token: Option<String>,

                        #[serde(flatten)]
                        page_contents: T,
                    }

                    if self.finished {
                        return None;
                    }

                    let paginated_result: PaginatedResult<T> = match self.method.execute() {
                        Ok(r) => r,
                        Err(err) => return Some(Err(err)),
                    };

                    if let Some(next_page_token) = paginated_result.next_page_token {
                        self.method.set_page_token(next_page_token);
                    } else {
                        self.finished = true;
                    }

                    Some(Ok(paginated_result.page_contents))
                }
            }

            fn auth_token() -> &'static str {
                #auth_token
            }

            pub mod schemas {
                #(#schemas_to_create)*
            }
            pub mod params {
                #(#params_to_create)*
            }
            pub struct Client{
                reqwest: ::reqwest::Client,
            }
            impl Client {
                pub fn new() -> Self {
                    Client{
                        reqwest: ::reqwest::Client::builder().timeout(None).build().unwrap(),
                    }
                }

                #(#resource_actions)*
            }
            #(#resource_modules)*
        }
    }

    fn all_types(&self) -> Vec<&Type> {
        fn add_types<'a>(typ: &'a Type, out: &mut Vec<&'a Type>) {
            match &typ.type_desc {
                TypeDesc::Array { items } => {
                    add_types(&items, out);
                }
                TypeDesc::Object { props, add_props } => {
                    for prop in props.values() {
                        add_types(&prop.typ, out);
                    }
                    if let Some(boxed_prop) = add_props {
                        add_types(&boxed_prop.typ, out);
                    }
                }
                _ => {}
            }
            out.push(typ);
        }
        fn add_resource_types<'a>(resource: &'a Resource, out: &mut Vec<&'a Type>) {
            for resource in &resource.resources {
                add_resource_types(resource, out);
            }
            for method in &resource.methods {
                for param in &method.params {
                    add_types(&param.typ, out);
                }
                if let Some(req) = method.request.as_ref() {
                    add_types(req, out);
                }
                if let Some(resp) = method.response.as_ref() {
                    add_types(resp, out);
                }
            }
        }
        let mut out = Vec::new();
        for typ in &self.schema_types {
            add_types(typ, &mut out);
        }
        for param in &self.params {
            add_types(&param.typ, &mut out);
        }
        for resource in &self.resources {
            add_resource_types(resource, &mut out);
        }
        let type_path_cmp = |a: &&Type, b: &&Type| {
            let a_path = a.type_path();
            let b_path = b.type_path();
            let a_path = quote! {#a_path}.to_string();
            let b_path = quote! {#b_path}.to_string();
            a_path.cmp(&b_path)
        };
        out.sort_by(type_path_cmp);
        out.dedup_by(|a, b| type_path_cmp(a, b) == std::cmp::Ordering::Equal);
        out
    }
}

#[derive(Clone, Debug)]
struct Resource {
    ident: syn::Ident,
    parent_path: syn::Path,
    resources: Vec<Resource>,
    methods: Vec<Method>,
}

impl Resource {
    fn from_disco_resource(
        resource_id: &str,
        parent_path: &syn::Path,
        disco_resource: &discovery_parser::ResourceDesc,
        all_schemas: &BTreeMap<String, discovery_parser::SchemaDesc>,
    ) -> Resource {
        let resource_ident = to_ident(&to_rust_varstr(&resource_id));
        let mut methods: Vec<Method> = disco_resource
            .methods
            .iter()
            .map(|(method_id, method_desc)| {
                Method::from_disco_method(
                    method_id,
                    &parse_quote! {crate::#resource_ident},
                    method_desc,
                    all_schemas,
                )
            })
            .collect();
        let mut nested_resources: Vec<Resource> = disco_resource
            .resources
            .iter()
            .map(|(nested_id, resource_desc)| {
                Resource::from_disco_resource(
                    nested_id,
                    &parse_quote! {parent_path::#resource_ident},
                    resource_desc,
                    all_schemas,
                )
            })
            .collect();
        methods.sort_by(|a, b| a.id.cmp(&b.id));
        nested_resources.sort_by(|a, b| a.ident.cmp(&b.ident));
        Resource {
            ident: resource_ident,
            parent_path: parent_path.clone(),
            resources: nested_resources,
            methods,
        }
    }

    fn action_type_name(&self) -> syn::Ident {
        to_ident(&to_rust_typestr(&format!("{}Actions", &self.ident)))
    }
}

#[derive(Clone, Debug)]
struct Method {
    id: String,
    path: String,
    http_method: String,
    description: Option<String>,
    param_order: Vec<String>,
    params: Vec<Param>,
    request: Option<Type>,
    response: Option<Type>,
    scopes: Vec<String>,
    supports_media_download: bool,
    media_upload: Option<MediaUpload>,
}

#[derive(Clone, Debug)]
struct MediaUpload {
    accept: Vec<String>,
    max_size: Option<String>,
    simple_path: Option<String>,
    resumable_path: Option<String>,
}

impl Method {
    fn from_disco_method(
        method_id: &str,
        parent_path: &syn::TypePath,
        disco_method: &discovery_parser::MethodDesc,
        all_schemas: &BTreeMap<String, discovery_parser::SchemaDesc>,
    ) -> Method {
        let request = disco_method.request.as_ref().map(|req| {
            let req_ident = to_ident(&to_rust_typestr(&format!("{}-request", method_id)));
            Type::from_disco_ref_or_type(
                &req_ident,
                &parse_quote! {#parent_path::schemas},
                req,
                all_schemas,
            )
        });
        let response = disco_method.response.as_ref().map(|resp| {
            let resp_ident = to_ident(&to_rust_typestr(&format!("{}-response", method_id)));
            Type::from_disco_ref_or_type(
                &resp_ident,
                &parse_quote! {#parent_path::schemas},
                resp,
                all_schemas,
            )
        });

        let mut params: Vec<Param> = disco_method
            .parameters
            .iter()
            .map(|(param_id, param_desc)| {
                Param::from_disco_method_param(
                    &method_id,
                    param_id,
                    &parse_quote! {#parent_path::params},
                    param_desc,
                )
            })
            .collect();
        // Sort params first by parameter order, then by ident.
        params.sort_by(|a, b| {
            let pos_in_param_order = |param: &Param| {
                disco_method
                    .parameter_order
                    .iter()
                    .position(|param_name| to_ident(&to_rust_varstr(param_name)) == param.ident)
            };
            let a_pos = pos_in_param_order(a);
            let b_pos = pos_in_param_order(b);
            match (a_pos, b_pos) {
                (Some(a), Some(b)) => a.cmp(&b),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.ident.cmp(&b.ident),
            }
        });

        let media_upload = disco_method.media_upload.as_ref().map(|media_upload| {
            use discovery_parser::UploadProtocol as DiscoUploadProtocol;
            let from_disco_upload_protocol = |&DiscoUploadProtocol {
                                                  ref multipart,
                                                  ref path,
                                              }| {
                if !multipart {
                    panic!("An upload protocol doesn't support multipart.");
                }
                // Many (all?) upload paths start with a '/' which when appended
                // with rootUrl will result in duplicate '/'s. Remove a starting
                // '/' in the upload path to address this.
                let path = if path.starts_with('/') {
                    &path[1..]
                } else {
                    path.as_str()
                };
                path.to_owned()
            };
            MediaUpload {
                accept: media_upload.accept.clone(),
                max_size: media_upload.max_size.clone(),
                simple_path: media_upload
                    .protocols
                    .simple
                    .as_ref()
                    .map(from_disco_upload_protocol),
                resumable_path: media_upload
                    .protocols
                    .resumable
                    .as_ref()
                    .map(from_disco_upload_protocol),
            }
        });

        Method {
            id: method_id.to_owned(),
            path: disco_method.path.clone(),
            http_method: disco_method.http_method.clone(),
            description: disco_method.description.clone(),
            param_order: disco_method.parameter_order.clone(),
            params,
            request,
            response,
            scopes: disco_method.scopes.clone(),
            supports_media_download: disco_method.supports_media_download,
            media_upload,
        }
    }

    fn builder_name(&self) -> syn::Ident {
        to_ident(&to_rust_typestr(&format!("{}-RequestBuilder", &self.id)))
    }
}

#[derive(Clone, Debug)]
struct Param {
    id: String,
    ident: syn::Ident,
    description: Option<String>,
    default: Option<String>,
    location: String,
    required: bool,
    typ: Type,
}

impl Param {
    fn from_disco_param(
        param_id: &str,
        parent_path: &syn::TypePath,
        disco_param: &discovery_parser::ParamDesc,
    ) -> Param {
        let ident = to_ident(&to_rust_varstr(&param_id));
        let type_ident = to_ident(&to_rust_typestr(&param_id));
        Param::with_ident(param_id, ident, type_ident, parent_path, disco_param)
    }

    fn from_disco_method_param(
        method_id: &str,
        param_id: &str,
        parent_path: &syn::TypePath,
        disco_param: &discovery_parser::ParamDesc,
    ) -> Param {
        let ident = to_ident(&to_rust_varstr(param_id));
        let type_ident = to_ident(&to_rust_typestr(&format!("{}-{}", &method_id, &param_id)));
        Param::with_ident(param_id, ident, type_ident, parent_path, disco_param)
    }

    fn with_ident(
        id: &str,
        ident: syn::Ident,
        type_ident: syn::Ident,
        parent_path: &syn::TypePath,
        disco_param: &discovery_parser::ParamDesc,
    ) -> Param {
        let typ = Type::from_disco_ref_or_type(
            &type_ident,
            parent_path,
            &RefOrType::Type(discovery_parser::TypeDesc::from_param(disco_param.clone())),
            &BTreeMap::new(),
        );
        Param {
            id: id.to_owned(),
            ident,
            description: disco_param.description.clone(),
            default: disco_param.default.clone(),
            location: disco_param.location.clone(),
            required: disco_param.required,
            typ,
        }
    }

    fn init_method(&self) -> ParamInitMethod {
        match self.typ.type_desc {
            TypeDesc::String => ParamInitMethod::IntoImpl(parse_quote! {String}),
            TypeDesc::Bool => ParamInitMethod::ByValue,
            TypeDesc::Int32 => ParamInitMethod::ByValue,
            TypeDesc::Uint32 => ParamInitMethod::ByValue,
            TypeDesc::Float32 => ParamInitMethod::ByValue,
            TypeDesc::Int64 => ParamInitMethod::ByValue,
            TypeDesc::Uint64 => ParamInitMethod::ByValue,
            TypeDesc::Float64 => ParamInitMethod::ByValue,
            TypeDesc::Bytes => ParamInitMethod::IntoImpl(parse_quote! {Box<[u8]>}),
            TypeDesc::Date => ParamInitMethod::IntoImpl(parse_quote! {String}),
            TypeDesc::DateTime => ParamInitMethod::IntoImpl(parse_quote! {String}),
            TypeDesc::Enum(_) => ParamInitMethod::ByValue,
            TypeDesc::Any | TypeDesc::Array { .. } | TypeDesc::Object { .. } => panic!(
                "param {} is not an expected type: {:?}",
                &self.ident, &self.typ.type_desc
            ),
        }
    }
}

#[derive(Clone, Debug)]
enum ParamInitMethod {
    IntoImpl(syn::TypePath),
    ByValue,
}

fn to_rust_typestr(s: &str) -> String {
    use inflector::cases::pascalcase::to_pascal_case;
    let s = to_pascal_case(s);
    fixup(s)
}

fn to_rust_varstr(s: &str) -> String {
    use inflector::cases::snakecase::to_snake_case;
    let s = to_snake_case(s);
    fixup(s)
}

fn fixup(s: String) -> String {
    // TODO: add all keywords
    if ["type", "match"].contains(&s.as_str()) {
        return format!("r#{}", s);
    }
    let s: String = s
        .chars()
        .map(|c| if !c.is_ascii_alphanumeric() { '_' } else { c })
        .collect();
    match s.chars().nth(0) {
        Some(c) if c.is_ascii_digit() => "_".to_owned() + &s,
        _ => s,
    }
}

fn to_ident(s: &str) -> syn::Ident {
    syn::parse_str(s).unwrap_or_else(|_| panic!("failed to make ident from: {}", s))
}

fn make_field(doc: &Option<String>, ident: &syn::Ident, ty: syn::Type) -> syn::Field {
    let mut attrs = Vec::new();
    if let Some(doc) = doc {
        let doc = syn::LitStr::new(doc, Span::call_site());
        use syn::parse::Parser;
        attrs = syn::Attribute::parse_outer
            .parse2(quote! {
                #[doc=#doc]
            })
            .expect("failed to parse doc string");
    }

    syn::Field {
        attrs,
        vis: syn::parse_quote! {pub},
        ident: Some(ident.clone()),
        colon_token: Some(syn::parse_quote! {:}),
        ty: parse_quote! {Option<#ty>},
    }
}

#[derive(Clone, Debug)]
struct Type {
    id: syn::PathSegment,
    parent_path: syn::TypePath,
    type_desc: TypeDesc,
}

impl Type {
    fn from_disco_schema(
        disco_schema: &discovery_parser::SchemaDesc,
        all_schemas: &BTreeMap<String, discovery_parser::SchemaDesc>,
    ) -> Type {
        let ident = to_ident(&to_rust_typestr(&disco_schema.id));
        Type::from_disco_ref_or_type(
            &ident,
            &parse_quote! {crate::schemas},
            &RefOrType::Type(disco_schema.typ.clone()),
            all_schemas,
        )
    }

    fn from_disco_ref_or_type(
        ident: &syn::Ident,
        parent_path: &syn::TypePath,
        ref_or_type: &RefOrType<discovery_parser::TypeDesc>,
        all_schemas: &BTreeMap<String, discovery_parser::SchemaDesc>,
    ) -> Type {
        let empty_type_path = || syn::TypePath {
            qself: None,
            path: syn::Path {
                leading_colon: None,
                segments: syn::punctuated::Punctuated::new(),
            },
        };
        match ref_or_type {
            RefOrType::Ref(reference) => {
                let reference_schema = all_schemas
                    .get(reference)
                    .unwrap_or_else(|| panic!("failed to lookup {} in schemas", reference));
                Type::from_disco_schema(reference_schema, all_schemas)
            }
            RefOrType::Type(disco_type) => {
                let type_desc =
                    TypeDesc::from_disco_type(ident, parent_path, disco_type, all_schemas);
                match type_desc {
                    TypeDesc::Any => unimplemented!("Any"),
                    TypeDesc::String => Type {
                        id: parse_quote! {String},
                        parent_path: empty_type_path(),
                        type_desc,
                    },
                    TypeDesc::Bool => Type {
                        id: parse_quote! {bool},
                        parent_path: empty_type_path(),
                        type_desc,
                    },
                    TypeDesc::Int32 => Type {
                        id: parse_quote! {i32},
                        parent_path: empty_type_path(),
                        type_desc,
                    },
                    TypeDesc::Uint32 => Type {
                        id: parse_quote! {u32},
                        parent_path: empty_type_path(),
                        type_desc,
                    },
                    TypeDesc::Float32 => Type {
                        id: parse_quote! {f32},
                        parent_path: empty_type_path(),
                        type_desc,
                    },
                    TypeDesc::Int64 => Type {
                        id: parse_quote! {i64},
                        parent_path: empty_type_path(),
                        type_desc,
                    },
                    TypeDesc::Uint64 => Type {
                        id: parse_quote! {u64},
                        parent_path: empty_type_path(),
                        type_desc,
                    },
                    TypeDesc::Float64 => Type {
                        id: parse_quote! {f64},
                        parent_path: empty_type_path(),
                        type_desc,
                    },
                    TypeDesc::Bytes => Type {
                        id: parse_quote! {Vec<u8>},
                        parent_path: empty_type_path(),
                        type_desc,
                    },
                    TypeDesc::Date => Type {
                        id: parse_quote! {Date<chrono::offset::Utc>},
                        parent_path: parse_quote! {::chrono},
                        type_desc,
                    },
                    TypeDesc::DateTime => Type {
                        id: parse_quote! {DateTime<chrono::offset::Utc>},
                        parent_path: parse_quote! {::chrono},
                        type_desc,
                    },
                    TypeDesc::Enum(_) => Type {
                        id: parse_quote! {#ident},
                        parent_path: parent_path.clone(),
                        type_desc,
                    },
                    TypeDesc::Array { ref items } => {
                        let item_path = items.type_path();
                        Type {
                            id: parse_quote! {Vec<#item_path>},
                            parent_path: empty_type_path(),
                            type_desc,
                        }
                    }
                    TypeDesc::Object {
                        ref props,
                        ref add_props,
                    } => {
                        let add_props_type = add_props.as_ref().map(|prop| prop.typ.type_path());
                        match (props.is_empty(), add_props_type) {
                            (true, Some(add_props_type)) => Type {
                                id: parse_quote! {BTreeMap<String, #add_props_type>},
                                parent_path: parse_quote! {::std::collections},
                                type_desc,
                            },
                            _ => Type {
                                id: parse_quote! {#ident},
                                parent_path: parent_path.clone(),
                                type_desc,
                            },
                        }
                    }
                }
            }
        }
    }

    fn type_path(&self) -> syn::TypePath {
        let id = &self.id;
        let parent_path = &self.parent_path;
        if parent_path.qself.is_none()
            && parent_path.path.leading_colon.is_none()
            && parent_path.path.segments.is_empty()
        {
            parse_quote! {#id}
        } else {
            parse_quote! {#parent_path::#id}
        }
    }

    fn type_path_str(&self) -> String {
        use quote::ToTokens;
        self.type_path().into_token_stream().to_string()
    }

    fn type_def(&self) -> Option<TokenStream> {
        let mut derives = vec![
            quote! {Debug},
            quote! {Clone},
            quote! {PartialEq},
            quote! {PartialOrd},
        ];
        if self.type_desc.is_hashable() {
            derives.push(quote! {Hash});
        }
        if self.type_desc.is_ord() {
            derives.push(quote! {Ord});
        }
        if self.type_desc.is_eq() {
            derives.push(quote! {Eq});
        }
        let name = &self.id;
        match &self.type_desc {
            TypeDesc::Enum(enums) => {
                derives.push(quote! {Copy});
                let variants = enums.iter().map(
                    |EnumDesc {
                         description, ident, ..
                     }| {
                        let doc: Option<TokenStream> = description.as_ref().map(|description| {
                            quote! {#[doc = #description]}
                        });
                        quote! {
                            #doc
                            #ident
                        }
                    },
                );
                let to_string_arms = enums.iter().map(|EnumDesc { ident, value, .. }| {
                    quote! {#name::#ident => #value}
                });
                let from_string_arms = enums.iter().map(|EnumDesc { ident, value, .. }| {
                    quote! {#value => #name::#ident}
                });

                Some(quote! {
                    #[derive(#(#derives,)*)]
                    pub enum #name {
                        #(#variants,)*
                    }

                    impl #name {
                        pub fn as_str(self) -> &'static str {
                            match self {
                                #(#to_string_arms,)*
                            }
                        }
                    }

                    impl ::std::fmt::Display for #name {
                        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                            f.write_str(self.as_str())
                        }
                    }

                    impl ::serde::Serialize for #name {
                        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                            where S: ::serde::ser::Serializer
                        {
                            serializer.serialize_str(self.as_str())
                        }
                    }

                    impl<'de> ::serde::Deserialize<'de> for #name {
                        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                        where
                            D: ::serde::de::Deserializer<'de>,
                        {
                            let value: &'de str = <&str>::deserialize(deserializer)?;
                            Ok(match value{
                                #(#from_string_arms,)*
                                _ => return Err(::serde::de::Error::custom(format!("invalid enum for #name: {}", value))),
                            })
                        }
                    }
                })
            }
            TypeDesc::Object { props, add_props } => match (props.is_empty(), add_props) {
                (false, add_props) => {
                    let mut fields: Vec<syn::Field> = props
                        .iter()
                        .map(
                            |(
                                _,
                                PropertyDesc {
                                    id,
                                    ident,
                                    description,
                                    typ,
                                    ..
                                },
                            )| {
                                use syn::parse::Parser;
                                let mut field = make_field(
                                    &description,
                                    ident,
                                    syn::Type::Path(typ.type_path()),
                                );
                                field.attrs.extend(
                                    syn::Attribute::parse_outer
                                        .parse2(quote! {
                                            #[serde(rename=#id,default)]
                                        })
                                        .expect("failed to parse serde attr"),
                                );
                                if let TypeDesc::Int64 | TypeDesc::Uint64 = typ.type_desc {
                                    field.attrs.extend(
                                        syn::Attribute::parse_outer
                                            .parse2(quote! {
                                                #[serde(with="crate::parsed_string")]
                                            })
                                            .expect("failed to parse serde attr"),
                                    );
                                }
                                field
                            },
                        )
                        .collect();
                    if let Some(boxed_prop_desc) = add_props.as_ref() {
                        let PropertyDesc {
                            ident,
                            description,
                            typ,
                            ..
                        } = &**boxed_prop_desc;
                        let add_props_type_path = typ.type_path();
                        let mut field = make_field(
                            &description,
                            &ident,
                            parse_quote! {BTreeMap<String, #add_props_type_path},
                        );
                        use syn::parse::Parser;
                        field.attrs.extend(
                            syn::Attribute::parse_outer
                                .parse2(quote! {
                                    #[serde(flatten)]
                                })
                                .expect("failed to parse flatten attr"),
                        );
                        fields.push(field);
                    }
                    derives.push(quote! {Default});
                    derives.push(quote! {::serde::Deserialize});
                    derives.push(quote! {::serde::Serialize});
                    Some(quote! {
                        #[derive(#(#derives,)*)]
                        pub struct #name {
                            #(#fields,)*
                        }

                        impl ::field_selector::FieldSelector for #name {
                            fn field_selector_with_ident(ident: &str, selector: &mut String) {
                                match selector.chars().rev().nth(0) {
                                    Some(',') | None => {},
                                    _ => selector.push_str(","),
                                }
                                selector.push_str(ident);
                                selector.push_str("*");
                            }
                        }
                    })
                }
                (true, Some(_)) => None,
                (true, None) => {
                    derives.push(quote! {Copy});
                    derives.push(quote! {Default});
                    derives.push(quote! {::serde::Deserialize});
                    derives.push(quote! {::serde::Serialize});
                    Some(quote! {
                        #[derive(#(#derives,)*)]
                        pub struct #name;

                        impl ::field_selector::FieldSelector for #name {
                            fn field_selector_with_ident(ident; &str, selector: &mut String) {}
                        }
                    })
                }
            },
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
enum TypeDesc {
    Any,
    String,
    Bool,
    Int32,
    Uint32,
    Float32,
    Int64,
    Uint64,
    Float64,
    Bytes,
    Date,
    DateTime,
    Enum(Vec<EnumDesc>),
    Array {
        items: Box<Type>,
    },
    Object {
        props: BTreeMap<syn::Ident, PropertyDesc>,
        add_props: Option<Box<PropertyDesc>>,
    },
}

impl TypeDesc {
    fn from_disco_type(
        ident: &syn::Ident,
        parent_path: &syn::TypePath,
        disco_type: &discovery_parser::TypeDesc,
        all_schemas: &BTreeMap<String, discovery_parser::SchemaDesc>,
    ) -> TypeDesc {
        match (
            disco_type.typ.as_str(),
            disco_type.format.as_ref().map(|x| x.as_str()),
        ) {
            ("any", None) => TypeDesc::Any,
            ("boolean", None) => TypeDesc::Bool,
            ("integer", Some("uint32")) => TypeDesc::Uint32,
            ("integer", Some("int32")) => TypeDesc::Int32,
            ("number", Some("float")) => TypeDesc::Float32,
            ("number", Some("double")) => TypeDesc::Float64,
            ("string", Some("int64")) => TypeDesc::Int64,
            ("string", Some("uint64")) => TypeDesc::Uint64,
            ("string", Some("byte")) => TypeDesc::Bytes,
            ("string", Some("date")) => TypeDesc::Date,
            ("string", Some("date-time")) => TypeDesc::DateTime,
            ("string", _) => {
                if disco_type.enumeration.is_empty() {
                    TypeDesc::String
                } else {
                    TypeDesc::Enum(
                        disco_type
                            .enumeration
                            .iter()
                            .zip(disco_type.enum_descriptions.iter())
                            .map(|(value, description)| {
                                let ident = to_ident(&to_rust_typestr(&value));
                                let description = if description.is_empty() {
                                    None
                                } else {
                                    Some(description.clone())
                                };
                                EnumDesc {
                                    ident,
                                    description,
                                    value: value.to_owned(),
                                }
                            })
                            .collect(),
                    )
                }
            }
            ("array", None) => {
                if let Some(ref items) = disco_type.items {
                    let items_ident = to_ident(&to_rust_typestr(&format!("{}-items", ident)));
                    let item_type = Type::from_disco_ref_or_type(
                        &items_ident,
                        &parent_path,
                        items,
                        all_schemas,
                    );
                    TypeDesc::Array {
                        items: Box::new(item_type),
                    }
                } else {
                    panic!("no items specified within array: {:?}", disco_type);
                }
            }
            ("object", None) => {
                use discovery_parser::PropertyDesc as DiscoPropDesc;
                let props = disco_type
                    .properties
                    .iter()
                    .map(|(prop_id, DiscoPropDesc { description, typ })| {
                        let prop_ident = to_ident(&to_rust_varstr(&prop_id));
                        let type_ident =
                            to_ident(&to_rust_typestr(&format!("{}-{}", ident, prop_id)));
                        let typ = Type::from_disco_ref_or_type(
                            &type_ident,
                            &parent_path,
                            &typ,
                            all_schemas,
                        );
                        (
                            prop_ident.clone(),
                            PropertyDesc {
                                id: prop_id.clone(),
                                ident: prop_ident,
                                description: description.clone(),
                                typ,
                            },
                        )
                    })
                    .collect();

                let add_props = disco_type.additional_properties.as_ref().map(|prop_desc| {
                    let prop_id = format!("{}-additional-properties", &ident);
                    let type_ident = to_ident(&to_rust_typestr(&prop_id));
                    let typ = Type::from_disco_ref_or_type(
                        &type_ident,
                        &parent_path,
                        &prop_desc.typ,
                        all_schemas,
                    );
                    Box::new(PropertyDesc {
                        id: prop_id,
                        ident: parse_quote! {additional_properties},
                        description: prop_desc.description.clone(),
                        typ,
                    })
                });
                TypeDesc::Object { props, add_props }
            }
            _ => panic!(
                "unable to determine type from discovery doc: {:?}",
                disco_type
            ),
        }
    }

    fn is_hashable(&self) -> bool {
        match self {
            TypeDesc::Any => unimplemented!("Any"),
            TypeDesc::String => true,
            TypeDesc::Bool => true,
            TypeDesc::Int32 => true,
            TypeDesc::Uint32 => true,
            TypeDesc::Float32 => false,
            TypeDesc::Int64 => true,
            TypeDesc::Uint64 => true,
            TypeDesc::Float64 => false,
            TypeDesc::Bytes => true,
            TypeDesc::Date => true,
            TypeDesc::DateTime => true,
            TypeDesc::Enum(_) => true,
            TypeDesc::Array { items } => items.type_desc.is_hashable(),
            TypeDesc::Object { props, add_props } => {
                add_props
                    .as_ref()
                    .map(|prop| prop.typ.type_desc.is_hashable())
                    .unwrap_or(true)
                    && props.values().all(|prop| prop.typ.type_desc.is_hashable())
            }
        }
    }

    fn is_ord(&self) -> bool {
        match self {
            TypeDesc::Any => unimplemented!("Any"),
            TypeDesc::String => true,
            TypeDesc::Bool => true,
            TypeDesc::Int32 => true,
            TypeDesc::Uint32 => true,
            TypeDesc::Float32 => false,
            TypeDesc::Int64 => true,
            TypeDesc::Uint64 => true,
            TypeDesc::Float64 => false,
            TypeDesc::Bytes => true,
            TypeDesc::Date => true,
            TypeDesc::DateTime => true,
            TypeDesc::Enum(_) => true,
            TypeDesc::Array { items } => items.type_desc.is_ord(),
            TypeDesc::Object { props, add_props } => {
                add_props
                    .as_ref()
                    .map(|prop| prop.typ.type_desc.is_ord())
                    .unwrap_or(true)
                    && props.values().all(|prop| prop.typ.type_desc.is_ord())
            }
        }
    }

    fn is_eq(&self) -> bool {
        match self {
            TypeDesc::Any => unimplemented!("Any"),
            TypeDesc::String => true,
            TypeDesc::Bool => true,
            TypeDesc::Int32 => true,
            TypeDesc::Uint32 => true,
            TypeDesc::Float32 => false,
            TypeDesc::Int64 => true,
            TypeDesc::Uint64 => true,
            TypeDesc::Float64 => false,
            TypeDesc::Bytes => true,
            TypeDesc::Date => true,
            TypeDesc::DateTime => true,
            TypeDesc::Enum(_) => true,
            TypeDesc::Array { items } => items.type_desc.is_eq(),
            TypeDesc::Object { props, add_props } => {
                add_props
                    .as_ref()
                    .map(|prop| prop.typ.type_desc.is_eq())
                    .unwrap_or(true)
                    && props.values().all(|prop| prop.typ.type_desc.is_eq())
            }
        }
    }
}

#[derive(Clone, Debug)]
struct PropertyDesc {
    id: String,
    ident: syn::Ident,
    description: Option<String>,
    typ: Type,
}

#[derive(Clone, Debug)]
struct EnumDesc {
    description: Option<String>,
    ident: syn::Ident,
    value: String,
}

fn any_method_supports_media(resources: &[Resource]) -> bool {
    resources.iter().any(|resource| {
        resource
            .methods
            .iter()
            .any(|method| method.supports_media_download || method.media_upload.is_some())
    })
}

fn add_media_to_alt_param(params: &mut [Param]) {
    if let Some(alt_param) = params.iter_mut().find(|p| p.id == "alt") {
        if let Param {
            typ:
                Type {
                    type_desc: TypeDesc::Enum(enum_desc),
                    ..
                },
            ..
        } = alt_param
        {
            if enum_desc.iter().find(|d| d.value == "media").is_none() {
                enum_desc.push(EnumDesc {
                    description: Some("Upload/Download media content".to_owned()),
                    ident: parse_quote! {Media},
                    value: "media".to_owned(),
                })
            }
        }
    }
}
