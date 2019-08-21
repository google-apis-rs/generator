#![recursion_limit = "256"] // for quote macro

use discovery_parser::{DiscoveryRestDesc, RefOrType as DiscoRefOrType};
use log::{debug, info};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use shared;
use std::{borrow::Cow, collections::BTreeMap, error::Error};
use syn::parse_quote;

mod cargo;
mod markdown;
mod method_actions;
mod method_builder;
mod path_templates;
mod resource_actions;
mod resource_builder;
mod rustfmt;

pub fn generate<P>(
    api_name: &str,
    discovery_desc: &DiscoveryRestDesc,
    base_dir: P,
) -> Result<(), Box<dyn Error>>
where
    P: AsRef<std::path::Path>,
{
    use std::io::Write;
    let constants = shared::Standard::default();
    let lib_path = base_dir.as_ref().join(constants.lib_path);
    let cargo_toml_path = base_dir.as_ref().join(constants.cargo_toml_path);

    info!("building api desc");
    let api_desc = APIDesc::from_discovery(discovery_desc);

    info!("creating directory and Cargo.toml");
    std::fs::create_dir_all(&lib_path.parent().expect("file in directory"))?;

    let cargo_contents = cargo::cargo_toml(api_name).to_string();
    std::fs::write(&cargo_toml_path, &cargo_contents)?;

    info!("writing lib '{}'", lib_path.display());
    let output_file = std::fs::File::create(&lib_path)?;
    let mut rustfmt_writer = crate::rustfmt::RustFmtWriter::new(output_file)?;
    rustfmt_writer.write_all(api_desc.generate().to_string().as_bytes())?;
    rustfmt_writer.write_all(include_bytes!("../gen_include/percent_encode_consts.rs"))?;
    rustfmt_writer.write_all(include_bytes!("../gen_include/multipart.rs"))?;
    rustfmt_writer.write_all(include_bytes!("../gen_include/resumable_upload.rs"))?;
    rustfmt_writer.write_all(include_bytes!("../gen_include/parsed_string.rs"))?;
    rustfmt_writer.write_all(include_bytes!("../gen_include/iter.rs"))?;
    rustfmt_writer.close()?;
    info!("done");
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
    schemas: BTreeMap<syn::Ident, Type>,
    params: Vec<Param>,
    resources: Vec<Resource>,
    methods: Vec<Method>,
}

impl APIDesc {
    fn from_discovery(discovery_desc: &DiscoveryRestDesc) -> APIDesc {
        debug!("collecting schema_types");
        let schemas: BTreeMap<syn::Ident, Type> = discovery_desc
            .schemas
            .iter()
            .map(|(id, schema)| (schema_id_to_ident(id), Type::from_disco_schema(schema)))
            .collect();
        debug!("collecting params");
        let mut params: Vec<Param> = discovery_desc
            .parameters
            .iter()
            .map(|(param_id, param_desc)| {
                Param::from_disco_param(param_id, &parse_quote! {crate::params}, param_desc)
            })
            .collect();
        debug!("collecting resources");
        let mut resources: Vec<Resource> = discovery_desc
            .resources
            .iter()
            .map(|(resource_id, resource_desc)| {
                Resource::from_disco_resource(
                    resource_id,
                    &parse_quote! {crate::resources},
                    resource_desc,
                )
            })
            .collect();
        debug!("collecting methods");
        let mut methods: Vec<Method> = discovery_desc
            .methods
            .iter()
            .map(|(method_id, method_desc)| {
                Method::from_disco_method(method_id, &parse_quote! {crate}, method_desc)
            })
            .collect();
        if any_method_supports_media(&resources) {
            add_media_to_alt_param(&mut params);
        }
        debug!("sorting");
        params.sort_by(|a, b| a.ident.cmp(&b.ident));
        resources.sort_by(|a, b| a.ident.cmp(&b.ident));
        methods.sort_by(|a, b| a.id.cmp(&b.id));
        APIDesc {
            name: discovery_desc.name.clone(),
            version: discovery_desc.version.clone(),
            root_url: discovery_desc.root_url.clone(),
            service_path: discovery_desc.service_path.clone(),
            schemas,
            params,
            resources,
            methods,
        }
    }

    fn generate(&self) -> TokenStream {
        info!("getting all types");
        let mut schema_type_defs = Vec::new();
        for (schema_id, schema) in self.schemas.iter() {
            if schema.type_def(&self.schemas).is_some() {
                // The schemas is a composite type (enum or struct) and may
                // contain other nested composite types. Add all nested type
                // defs.
                append_nested_type_defs(
                    &RefOrType::Type(Cow::Borrowed(schema)),
                    &self.schemas,
                    &mut schema_type_defs,
                );
            } else {
                // The schema is a simple type that doesn't require creating a
                // new composite type, but we still create a type alias for it
                // so that creating a path from a Ref will point to a valid
                // type.
                let type_path = schema.type_path();
                schema_type_defs.push(quote! {type #schema_id = #type_path;});
            }
        }
        let mut param_type_defs = Vec::new();
        for param in &self.params {
            append_nested_type_defs(
                &RefOrType::Type(Cow::Borrowed(&param.typ)),
                &self.schemas,
                &mut param_type_defs,
            );
        }
        info!("generating resources");
        let resource_modules = self.resources.iter().map(|resource| {
            resource_builder::generate(
                &self.root_url,
                &self.service_path,
                &self.params,
                resource,
                &self.schemas,
            )
        });
        info!("creating resource actions");
        let resource_actions = self
            .resources
            .iter()
            .map(|resource| resource_actions::generate(resource));

        let method_builders = self.methods.iter().map(|method| {
            method_builder::generate(
                &self.root_url,
                &self.service_path,
                &self.params,
                method,
                &self.schemas,
            )
        });
        let method_actions = self
            .methods
            .iter()
            .map(|method| method_actions::generate(method, &self.params));
        info!("outputting");
        quote! {
            pub mod schemas {
                #(#schema_type_defs)*
            }
            pub mod params {
                #(#param_type_defs)*
            }
            pub struct Client<A> {
                reqwest: ::reqwest::Client,
                auth: ::std::sync::Mutex<A>,
            }
            impl<A: yup_oauth2::GetToken> Client<A> {
                pub fn new(auth: A) -> Self {
                    Client {
                        reqwest: ::reqwest::Client::builder().timeout(None).build().unwrap(),
                        auth: ::std::sync::Mutex::new(auth),
                    }
                }

                #(#resource_actions)*
                #(#method_actions)*
            }
            mod resources {
                #(#resource_modules)*
                #(#method_builders)*
            }
        }
    }
}

fn schema_id_to_ident(id: &str) -> syn::Ident {
    to_ident(&to_rust_typestr(id))
}

fn schema_parent_path() -> syn::Path {
    parse_quote! {crate::schemas}
}

fn append_nested_type_defs(
    ref_or_type: &RefOrType,
    schemas: &BTreeMap<syn::Ident, Type>,
    out: &mut Vec<TokenStream>,
) {
    fn add_type(typ: &Type, schemas: &BTreeMap<syn::Ident, Type>, out: &mut Vec<TokenStream>) {
        match &typ.type_desc {
            TypeDesc::Array { items } => {
                append_nested_type_defs(&items, schemas, out);
            }
            TypeDesc::Object { props, add_props } => {
                for prop in props.values() {
                    append_nested_type_defs(&prop.typ, schemas, out);
                }
                if let Some(boxed_prop) = add_props {
                    append_nested_type_defs(&boxed_prop.typ, schemas, out);
                }
            }
            _ => {}
        };
        if let Some(type_def) = typ.type_def(schemas) {
            out.push(type_def);
        }
    }
    match ref_or_type {
        RefOrType::Ref(_) => {}
        RefOrType::Type(typ) => add_type(typ, schemas, out),
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
    ) -> Resource {
        let resource_ident = to_ident(&to_rust_varstr(&resource_id));
        let mut methods: Vec<Method> = disco_resource
            .methods
            .iter()
            .map(|(method_id, method_desc)| {
                Method::from_disco_method(
                    method_id,
                    &parse_quote! {#parent_path::#resource_ident},
                    method_desc,
                )
            })
            .collect();
        let mut nested_resources: Vec<Resource> = disco_resource
            .resources
            .iter()
            .map(|(nested_id, resource_desc)| {
                Resource::from_disco_resource(
                    nested_id,
                    &parse_quote! {#parent_path::#resource_ident},
                    resource_desc,
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
    request: Option<RefOrType<'static>>,
    response: Option<RefOrType<'static>>,
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
    ) -> Method {
        let request = disco_method.request.as_ref().map(|req| {
            let req_ident = to_ident(&to_rust_typestr(&format!("{}-request", method_id)));
            RefOrType::from_disco_ref_or_type(
                &req_ident,
                &parse_quote! {#parent_path::schemas},
                req,
            )
        });
        let response = disco_method.response.as_ref().map(|resp| {
            let resp_ident = to_ident(&to_rust_typestr(&format!("{}-response", method_id)));
            RefOrType::from_disco_ref_or_type(
                &resp_ident,
                &parse_quote! {#parent_path::schemas},
                resp,
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
        parent_path: &syn::Path,
        disco_param: &discovery_parser::ParamDesc,
    ) -> Param {
        let ident = to_ident(&to_rust_varstr(&param_id));
        let type_ident = to_ident(&to_rust_typestr(&param_id));
        Param::with_ident(param_id, ident, type_ident, parent_path, disco_param)
    }

    fn from_disco_method_param(
        method_id: &str,
        param_id: &str,
        parent_path: &syn::Path,
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
        parent_path: &syn::Path,
        disco_param: &discovery_parser::ParamDesc,
    ) -> Param {
        let typ = Type::from_disco_type(
            &type_ident,
            parent_path,
            &discovery_parser::TypeDesc::from_param(disco_param.clone()),
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
        match &self.typ.type_desc {
            TypeDesc::String => ParamInitMethod::IntoImpl(parse_quote! {String}),
            TypeDesc::Bool => ParamInitMethod::ByValue,
            TypeDesc::Int32 => ParamInitMethod::ByValue,
            TypeDesc::Uint32 => ParamInitMethod::ByValue,
            TypeDesc::Float32 => ParamInitMethod::ByValue,
            TypeDesc::Int64 => ParamInitMethod::ByValue,
            TypeDesc::Uint64 => ParamInitMethod::ByValue,
            TypeDesc::Float64 => ParamInitMethod::ByValue,
            TypeDesc::Bytes => ParamInitMethod::IntoImpl(parse_quote! {Box<[u8]>}),
            TypeDesc::Date => ParamInitMethod::ByValue,
            TypeDesc::DateTime => ParamInitMethod::ByValue,
            TypeDesc::Enum(_) => ParamInitMethod::ByValue,
            TypeDesc::Array { items } => {
                let items_type_path = items.type_path();
                ParamInitMethod::IntoImpl(parse_quote! { Vec<#items_type_path> })
            }
            TypeDesc::Any | TypeDesc::Object { .. } => panic!(
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
    if [
        "as", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false",
        "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
        "ref", "return", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while", "abstract", "become", "box", "do", "final", "macro", "override",
        "priv", "typeof", "unsized", "virtual", "yield", "async", "await", "try",
    ]
    .contains(&s.as_str())
    {
        return format!("r#{}", s);
    }

    if &s == "self" {
        return "_self".to_owned();
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
        let doc = syn::LitStr::new(&markdown::sanitize(&doc), Span::call_site());
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
enum RefOrType<'a> {
    Ref(syn::Ident),
    Type(Cow<'a, Type>),
}

impl<'a> From<Type> for Cow<'a, Type> {
    fn from(t: Type) -> Self {
        Cow::Owned(t)
    }
}

impl<'a> From<&'a Type> for Cow<'a, Type> {
    fn from(t: &'a Type) -> Self {
        Cow::Borrowed(t)
    }
}

impl<'a> RefOrType<'a> {
    fn from_disco_ref_or_type(
        ident: &syn::Ident,
        parent_path: &syn::Path,
        ref_or_type: &DiscoRefOrType<discovery_parser::TypeDesc>,
    ) -> RefOrType<'static> {
        debug!("from_disco_ref_or_type({})", ident);
        match ref_or_type {
            DiscoRefOrType::Ref(reference) => RefOrType::Ref(schema_id_to_ident(reference)),
            DiscoRefOrType::Type(disco_type) => {
                RefOrType::Type(Type::from_disco_type(ident, parent_path, disco_type).into())
            }
        }
    }

    fn type_path(&self) -> syn::TypePath {
        match self {
            RefOrType::Ref(ident) => {
                let parent_path = schema_parent_path();
                parse_quote! {#parent_path::#ident}
            }
            RefOrType::Type(typ) => typ.type_path(),
        }
    }

    fn get_type(&'a self, schemas: &'a BTreeMap<syn::Ident, Type>) -> &'a Type {
        match self {
            RefOrType::Ref(reference) => schemas.get(reference).unwrap(),
            RefOrType::Type(typ) => typ.as_ref(),
        }
    }
}

#[derive(Clone, Debug)]
struct Type {
    id: syn::PathSegment,
    parent_path: syn::Path,
    type_desc: TypeDesc,
}

impl Type {
    fn from_disco_schema(disco_schema: &discovery_parser::SchemaDesc) -> Type {
        Type::from_disco_type(
            &schema_id_to_ident(&disco_schema.id),
            &schema_parent_path(),
            &disco_schema.typ,
        )
    }

    fn from_disco_type(
        ident: &syn::Ident,
        parent_path: &syn::Path,
        disco_type: &discovery_parser::TypeDesc,
    ) -> Type {
        let empty_type_path = syn::Path {
            leading_colon: None,
            segments: syn::punctuated::Punctuated::new(),
        };
        let type_desc = TypeDesc::from_disco_type(ident, parent_path, disco_type);
        match type_desc {
            TypeDesc::Any => Type {
                id: parse_quote! {Value},
                parent_path: parse_quote! {::serde_json},
                type_desc,
            },
            TypeDesc::String => Type {
                id: parse_quote! {String},
                parent_path: empty_type_path.clone(),
                type_desc,
            },
            TypeDesc::Bool => Type {
                id: parse_quote! {bool},
                parent_path: empty_type_path.clone(),
                type_desc,
            },
            TypeDesc::Int32 => Type {
                id: parse_quote! {i32},
                parent_path: empty_type_path.clone(),
                type_desc,
            },
            TypeDesc::Uint32 => Type {
                id: parse_quote! {u32},
                parent_path: empty_type_path.clone(),
                type_desc,
            },
            TypeDesc::Float32 => Type {
                id: parse_quote! {f32},
                parent_path: empty_type_path.clone(),
                type_desc,
            },
            TypeDesc::Int64 => Type {
                id: parse_quote! {i64},
                parent_path: empty_type_path.clone(),
                type_desc,
            },
            TypeDesc::Uint64 => Type {
                id: parse_quote! {u64},
                parent_path: empty_type_path.clone(),
                type_desc,
            },
            TypeDesc::Float64 => Type {
                id: parse_quote! {f64},
                parent_path: empty_type_path.clone(),
                type_desc,
            },
            TypeDesc::Bytes => Type {
                id: parse_quote! {Vec<u8>},
                parent_path: empty_type_path.clone(),
                type_desc,
            },
            TypeDesc::Date => Type {
                id: parse_quote! {NaiveDate},
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
                    parent_path: empty_type_path.clone(),
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

    fn type_path(&self) -> syn::TypePath {
        let id = &self.id;
        let parent_path = &self.parent_path;
        if parent_path.leading_colon.is_none() && parent_path.segments.is_empty() {
            parse_quote! {#id}
        } else {
            parse_quote! {#parent_path::#id}
        }
    }

    fn type_def(&self, schemas: &BTreeMap<syn::Ident, Type>) -> Option<TokenStream> {
        let mut derives = vec![quote! {Debug}, quote! {Clone}, quote! {PartialEq}];
        if self.nested_type_desc_fold(schemas, true, |accum, typ| accum && typ.is_hashable()) {
            derives.push(quote! {Hash});
        }
        if self.nested_type_desc_fold(schemas, true, |accum, typ| accum && typ.is_partial_ord()) {
            derives.push(quote! {PartialOrd});
        }
        if self.nested_type_desc_fold(schemas, true, |accum, typ| accum && typ.is_ord()) {
            derives.push(quote! {Ord});
        }
        if self.nested_type_desc_fold(schemas, true, |accum, typ| accum && typ.is_eq()) {
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
                            let description = markdown::sanitize(description);
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

                    impl ::field_selector::FieldSelector for #name {
                        fn field_selector_with_ident(ident: &str, selector: &mut String) {
                            match selector.chars().rev().nth(0) {
                                Some(',') | None => {},
                                _ => selector.push_str(","),
                            }
                            selector.push_str(ident);
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
                                    typ: ref_or_type,
                                    ..
                                },
                            )| {
                                use syn::parse::Parser;
                                let typ = ref_or_type.get_type(schemas);
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
                            fn field_selector_with_ident(_ident: &str, _selector: &mut String) {}
                        }
                    })
                }
            },
            _ => None,
        }
    }

    fn nested_type_desc_fold<F, B>(&self, schemas: &BTreeMap<syn::Ident, Type>, init: B, f: F) -> B
    where
        F: FnMut(B, &TypeDesc) -> B + Copy,
    {
        fn _nested_ref_or_type<'a, F, B>(
            ref_or_type: &'a RefOrType,
            schemas: &'a BTreeMap<syn::Ident, Type>,
            init: B,
            f: F,
            already_seen: &'a [&'a syn::Ident],
        ) -> B
        where
            F: FnMut(B, &TypeDesc) -> B + Copy,
        {
            match ref_or_type {
                RefOrType::Ref(reference) => {
                    if already_seen.iter().find(|&x| x == &reference).is_none() {
                        let mut already_seen = already_seen.to_vec();
                        already_seen.push(reference);
                        let typ = schemas.get(reference).unwrap();
                        _nested_type(typ, schemas, init, f, &already_seen)
                    } else {
                        init
                    }
                }
                RefOrType::Type(typ) => _nested_type(typ, schemas, init, f, already_seen),
            }
        }
        fn _nested_type<'a, F, B>(
            typ: &'a Type,
            schemas: &'a BTreeMap<syn::Ident, Type>,
            mut init: B,
            mut f: F,
            already_seen: &'a [&'a syn::Ident],
        ) -> B
        where
            F: FnMut(B, &TypeDesc) -> B + Copy,
        {
            match &typ.type_desc {
                TypeDesc::Any
                | TypeDesc::String
                | TypeDesc::Bool
                | TypeDesc::Int32
                | TypeDesc::Uint32
                | TypeDesc::Float32
                | TypeDesc::Int64
                | TypeDesc::Uint64
                | TypeDesc::Float64
                | TypeDesc::Bytes
                | TypeDesc::Date
                | TypeDesc::DateTime
                | TypeDesc::Enum(_) => f(init, &typ.type_desc),
                TypeDesc::Array { items } => {
                    _nested_ref_or_type(items, schemas, init, f, already_seen)
                }
                TypeDesc::Object { props, add_props } => {
                    if let Some(prop) = add_props {
                        init = _nested_ref_or_type(&prop.typ, schemas, init, f, already_seen);
                    }

                    for prop in props.values() {
                        init = _nested_ref_or_type(&prop.typ, schemas, init, f, already_seen);
                    }
                    init
                }
            }
        }
        _nested_type(self, schemas, init, f, &Vec::new())
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
        items: Box<RefOrType<'static>>,
    },
    Object {
        props: BTreeMap<syn::Ident, PropertyDesc>,
        add_props: Option<Box<PropertyDesc>>,
    },
}

impl TypeDesc {
    fn from_disco_type(
        ident: &syn::Ident,
        parent_path: &syn::Path,
        disco_type: &discovery_parser::TypeDesc,
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
                            .zip(
                                disco_type
                                    .enum_descriptions
                                    .iter()
                                    .map(|desc| {
                                        if desc.is_empty() {
                                            None
                                        } else {
                                            Some(desc.clone())
                                        }
                                    })
                                    .chain(std::iter::repeat(None)),
                            )
                            .map(|(value, description)| {
                                let ident = to_ident(&to_rust_typestr(&value));
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
                    let item_type =
                        RefOrType::from_disco_ref_or_type(&items_ident, &parent_path, items);
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
                        let ref_or_type =
                            RefOrType::from_disco_ref_or_type(&type_ident, &parent_path, &typ);
                        (
                            prop_ident.clone(),
                            PropertyDesc {
                                id: prop_id.clone(),
                                ident: prop_ident,
                                description: description.clone(),
                                typ: ref_or_type,
                            },
                        )
                    })
                    .collect();

                let add_props = disco_type.additional_properties.as_ref().map(|prop_desc| {
                    let prop_id = format!("{}-additional-properties", &ident);
                    let type_ident = to_ident(&to_rust_typestr(&prop_id));
                    let ref_or_type = RefOrType::from_disco_ref_or_type(
                        &type_ident,
                        &parent_path,
                        &prop_desc.typ,
                    );
                    Box::new(PropertyDesc {
                        id: prop_id,
                        ident: parse_quote! {additional_properties},
                        description: prop_desc.description.clone(),
                        typ: ref_or_type,
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
            TypeDesc::Any => false,
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
            TypeDesc::Array { .. } | TypeDesc::Object { .. } => {
                panic!("is_hashable should only be called on non-composite types")
            }
        }
    }

    fn is_ord(&self) -> bool {
        match self {
            TypeDesc::Any => false,
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
            TypeDesc::Array { .. } | TypeDesc::Object { .. } => {
                panic!("is_ord should only be called on non-composite types")
            }
        }
    }

    fn is_partial_ord(&self) -> bool {
        match self {
            TypeDesc::Any => false,
            TypeDesc::String => true,
            TypeDesc::Bool => true,
            TypeDesc::Int32 => true,
            TypeDesc::Uint32 => true,
            TypeDesc::Float32 => true,
            TypeDesc::Int64 => true,
            TypeDesc::Uint64 => true,
            TypeDesc::Float64 => true,
            TypeDesc::Bytes => true,
            TypeDesc::Date => true,
            TypeDesc::DateTime => true,
            TypeDesc::Enum(_) => true,
            TypeDesc::Array { .. } | TypeDesc::Object { .. } => {
                panic!("is_ord should only be called on non-composite types")
            }
        }
    }

    fn is_eq(&self) -> bool {
        match self {
            TypeDesc::Any => false,
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
            TypeDesc::Array { .. } | TypeDesc::Object { .. } => {
                panic!("is_eq should only be called on non-composite types")
            }
        }
    }
}

#[derive(Clone, Debug)]
struct PropertyDesc {
    id: String,
    ident: syn::Ident,
    description: Option<String>,
    typ: RefOrType<'static>,
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
