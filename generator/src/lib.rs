use discovery_parser::{DiscoveryRestDesc, RefOrType};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::collections::BTreeMap;
use std::error::Error;
use syn::parse_quote;

mod cargo;
mod method_builder;
mod path_templates;
mod resource_builder;

pub fn generate<U, P>(discovery_url: U, base_dir: P) -> Result<TokenStream, Box<dyn Error>>
where
    U: reqwest::IntoUrl,
    P: AsRef<std::path::Path>,
{
    let desc: DiscoveryRestDesc = reqwest::get(discovery_url)?.json()?;
    let api_desc = APIDesc::from_discovery(&desc);
    let project_path = base_dir.as_ref().join("foo");
    let src_path = project_path.join("src");
    std::fs::create_dir_all(&src_path)?;
    let cargo_path = project_path.join("Cargo.toml");
    let cargo_contents = toml::ser::to_string_pretty(&cargo::manifest(format!(
        "google_{}{}",
        &desc.name, &desc.version
    )))?;
    std::fs::write(&cargo_path, &cargo_contents)?;
    std::fs::write(&src_path.join("lib.rs"), &quote! {#api_desc}.to_string())?;
    Ok(quote! {#api_desc})
}

// A structure that represents the desired rust API. Typically built by
// transforming a discovery_parser::DiscoveryRestDesc.
#[derive(Clone, Debug)]
struct APIDesc {
    name: String,
    version: String,
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
        schema_types.sort_by(|a, b| a.type_path_str().cmp(&b.type_path_str()));
        params.sort_by(|a, b| a.ident.cmp(&b.ident));
        resources.sort_by(|a, b| a.ident.cmp(&b.ident));
        APIDesc {
            name: discovery_desc.name.clone(),
            version: discovery_desc.version.clone(),
            schema_types,
            params,
            resources,
        }
    }

    fn all_types(&self) -> Vec<Type> {
        fn add_types(typ: &Type, out: &mut Vec<Type>) {
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
            out.push(typ.clone());
        }
        fn add_resource_types(resource: &Resource, out: &mut Vec<Type>) {
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
        let type_path_cmp = |a: &Type, b: &Type| {
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

impl quote::ToTokens for APIDesc {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use quote::TokenStreamExt;

        let all_types = self.all_types();
        let schemas_to_create = all_types
            .iter()
            .filter(|typ| typ.parent_path == parse_quote! {crate::schemas})
            .filter_map(|typ| typ.type_def());
        let params_to_create = all_types
            .iter()
            .filter(|typ| typ.parent_path == parse_quote! {crate::params})
            .filter_map(|typ| typ.type_def());
        let resource_modules = self.resources.iter().map(resource_builder::generate);
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
                    crate::#resource_ident::#action_ident
                }
            }
        });
        tokens.append_all(std::iter::once(quote! {
            pub mod schemas {
                #(#schemas_to_create)*
            }
            pub mod params {
                #(#params_to_create)*
            }
            pub struct Client;
            impl Client {
                #(#resource_actions)*
            }
            #(#resource_modules)*
        }));
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
    let s = if ["type", "match"].contains(&s.as_str()) {
        format!("r#{}", s)
    } else {
        s
    };
    let s: String = s
        .chars()
        .map(|c| if !c.is_ascii_alphanumeric() { '_' } else { c })
        .collect();
    match s.chars().next() {
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
        ty,
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
                        id: parse_quote! {Date<chrono::UTC>},
                        parent_path: parse_quote! {::chrono},
                        type_desc,
                    },
                    TypeDesc::DateTime => Type {
                        id: parse_quote! {DateTime<chrono::UTC>},
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
                    TypeDesc::Object { .. } => Type {
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

    fn type_def(&self) -> Option<TypeDef> {
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
        match &self.type_desc {
            TypeDesc::Enum(enums) => {
                let variants = enums
                    .iter()
                    .map(|EnumDesc { description, ident }| {
                        let doc: Option<TokenStream> = description.as_ref().map(|description| {
                            parse_quote! {#[doc = #description]}
                        });
                        parse_quote! {
                            #doc
                            #ident
                        }
                    })
                    .collect();
                Some(TypeDef {
                    id: self.id.clone(),
                    parent_path: self.parent_path.clone(),
                    derives,
                    typ: TypeFields::Enum { variants },
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
                                    ident,
                                    description,
                                    typ,
                                },
                            )| {
                                make_field(&description, ident, syn::Type::Path(typ.type_path()))
                            },
                        )
                        .collect();
                    if let Some(boxed_prop_desc) = add_props.as_ref() {
                        let PropertyDesc {
                            ident,
                            description,
                            typ,
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
                    Some(TypeDef {
                        id: self.id.clone(),
                        parent_path: self.parent_path.clone(),
                        derives,
                        typ: TypeFields::Struct { fields },
                    })
                }
                (true, Some(_)) => None,
                (true, None) => Some(TypeDef {
                    id: self.id.clone(),
                    parent_path: self.parent_path.clone(),
                    derives,
                    typ: TypeFields::Struct { fields: Vec::new() },
                }),
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
            ("any", None) => unimplemented!("Any"),
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
                                EnumDesc { ident, description }
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
                                ident: prop_ident,
                                description: description.clone(),
                                typ,
                            },
                        )
                    })
                    .collect();

                let add_props = disco_type.additional_properties.as_ref().map(|prop_desc| {
                    let type_ident = to_ident(&to_rust_typestr(&format!(
                        "{}-additional-properties",
                        &ident
                    )));
                    let typ = Type::from_disco_ref_or_type(
                        &type_ident,
                        &parent_path,
                        &prop_desc.typ,
                        all_schemas,
                    );
                    Box::new(PropertyDesc {
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
    ident: syn::Ident,
    description: Option<String>,
    typ: Type,
}

#[derive(Clone, Debug)]
struct EnumDesc {
    description: Option<String>,
    ident: syn::Ident,
}

#[derive(Clone, Debug)]
struct TypeDef {
    id: syn::PathSegment,       // ident of this type e.g. MyType
    parent_path: syn::TypePath, // path to containing module e.g. types
    derives: Vec<TokenStream>,  // The derives to create
    typ: TypeFields,            // definition of this type
}

impl quote::ToTokens for TypeDef {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use quote::TokenStreamExt;
        let name = &self.id;
        let derives = &self.derives;
        tokens.append_all(std::iter::once(match &self.typ {
            TypeFields::Enum { variants } => {
                quote! {
                    #[derive(#(#derives,)*)]
                    pub enum #name {
                        #(#variants,)*
                    }
                }
            }
            TypeFields::Struct { fields } => {
                quote! {
                    #[derive(#(#derives,)*)]
                    pub struct #name {
                        #(#fields,)*
                    }
                }
            }
        }));
    }
}

#[derive(Clone, Debug)]
enum TypeFields {
    Enum { variants: Vec<syn::Variant> },
    Struct { fields: Vec<syn::Field> },
}
