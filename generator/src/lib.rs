use discovery_parser::{DiscoveryRestDesc, RefOrType};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::collections::HashMap;
use std::error::Error;
use syn::parse_quote;

mod resource_builder;
mod method_builder;

pub fn generate<U>(discovery_url: U) -> Result<TokenStream, Box<dyn Error>>
where
    U: reqwest::IntoUrl,
{
    let desc: DiscoveryRestDesc = reqwest::get(discovery_url)?.json()?;
    let api_desc = APIDesc::from_discovery(&desc);
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
                Resource::from_disco_resource(resource_id, resource_desc, &discovery_desc.schemas)
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
        let mut out = Vec::new();
        for typ in &self.schema_types {
            add_types(typ, &mut out);
        }
        for param in &self.params {
            add_types(&param.typ, &mut out);
        }
        for resource in &self.resources {
            for method in &resource.methods {
                for param in &method.params {
                    add_types(&param.typ, &mut out);
                }
                if let Some(req) = method.request.as_ref() {
                    add_types(req, &mut out);
                }
                if let Some(resp) = method.response.as_ref() {
                    add_types(resp, &mut out);
                }
            }
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
        let schemas_to_create = all_types.iter().filter(|typ| {
            typ.parent_path == parse_quote!{crate::schemas}
        }).filter_map(|typ| typ.type_def());
        let params_to_create = all_types.iter().filter(|typ| {
            typ.parent_path == parse_quote!{crate::params}
        }).filter_map(|typ| typ.type_def());
        let resource_modules = self.resources.iter().map(resource_builder::generate);
        tokens.append_all(std::iter::once(quote!{
            mod schemas {
                #(#schemas_to_create)*
            }
            mod params {
                #(#params_to_create)*
            }
            #(#resource_modules)*
        }));
    }
}

#[derive(Clone, Debug)]
struct Resource {
    ident: syn::Ident,
    methods: Vec<Method>,
}

impl Resource {
    fn from_disco_resource(
        resource_id: &str,
        disco_resource: &discovery_parser::ResourceDesc,
        all_schemas: &HashMap<String, discovery_parser::SchemaDesc>,
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
        methods.sort_by(|a, b| a.id.cmp(&b.id));
        Resource {
            ident: resource_ident,
            methods,
        }
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
        all_schemas: &HashMap<String, discovery_parser::SchemaDesc>,
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
                Param::from_disco_method_param(&method_id, param_id, &parse_quote! {#parent_path::params}, param_desc)
            })
            .collect();
        params.sort_by(|a, b| a.ident.cmp(&b.ident));
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
}

#[derive(Clone, Debug)]
struct Param {
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
        let ident = to_ident(&to_rust_typestr(&param_id));
        Param::with_ident(ident, parent_path, disco_param)
    }

    fn from_disco_method_param(
        method_id: &str,
        param_id: &str,
        parent_path: &syn::TypePath,
        disco_param: &discovery_parser::ParamDesc,
    ) -> Param {
        let ident = to_ident(&to_rust_typestr(&format!("{}-{}", &method_id, &param_id)));
        Param::with_ident(ident, parent_path, disco_param)
    }

    fn with_ident(
        ident: syn::Ident,
        parent_path: &syn::TypePath,
        disco_param: &discovery_parser::ParamDesc,
    ) -> Param {
        let typ = Type::from_disco_ref_or_type(
            &ident,
            parent_path,
            &RefOrType::Type(disco_param.typ.as_type_desc()),
            &HashMap::new(),
        );
        Param {
            ident,
            description: disco_param.description.clone(),
            default: disco_param.default.clone(),
            location: disco_param.location.clone(),
            required: disco_param.required,
            typ,
        }
    }
}

fn to_rust_typestr(s: &str) -> String {
    use inflector::cases::pascalcase::to_pascal_case;
    let s = to_pascal_case(s);
    escape_keywords(s)
}

fn to_rust_varstr(s: &str) -> String {
    use inflector::cases::snakecase::to_snake_case;
    let s = to_snake_case(s);
    escape_keywords(s)
}

fn escape_keywords(s: String) -> String {
    if ["type", "match"].contains(&s.as_str()) {
        format!("r#{}", s)
    } else {
        s
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
        all_schemas: &HashMap<String, discovery_parser::SchemaDesc>,
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
        all_schemas: &HashMap<String, discovery_parser::SchemaDesc>,
    ) -> Type {
        use discovery_parser::TypeDesc as DiscoTypeDesc;
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
            RefOrType::Type(DiscoTypeDesc::Any) => unimplemented!("Any"),
            RefOrType::Type(DiscoTypeDesc::String)
            | RefOrType::Type(DiscoTypeDesc::FormattedString { .. }) => Type {
                id: parse_quote! {String},
                parent_path: empty_type_path(),
                type_desc: TypeDesc::String,
            },
            RefOrType::Type(DiscoTypeDesc::Boolean) => Type {
                id: parse_quote! {bool},
                parent_path: empty_type_path(),
                type_desc: TypeDesc::Bool,
            },
            RefOrType::Type(DiscoTypeDesc::Int32) => Type {
                id: parse_quote! {i32},
                parent_path: empty_type_path(),
                type_desc: TypeDesc::Int32,
            },
            RefOrType::Type(DiscoTypeDesc::Uint32) => Type {
                id: parse_quote! {u32},
                parent_path: empty_type_path(),
                type_desc: TypeDesc::Uint32,
            },
            RefOrType::Type(DiscoTypeDesc::Float32) => Type {
                id: parse_quote! {f32},
                parent_path: empty_type_path(),
                type_desc: TypeDesc::Float32,
            },
            RefOrType::Type(DiscoTypeDesc::Int64) => Type {
                id: parse_quote! {i64},
                parent_path: empty_type_path(),
                type_desc: TypeDesc::Int64,
            },
            RefOrType::Type(DiscoTypeDesc::Uint64) => Type {
                id: parse_quote! {u64},
                parent_path: empty_type_path(),
                type_desc: TypeDesc::Uint64,
            },
            RefOrType::Type(DiscoTypeDesc::Float64) => Type {
                id: parse_quote! {f64},
                parent_path: empty_type_path(),
                type_desc: TypeDesc::Float64,
            },
            RefOrType::Type(DiscoTypeDesc::Bytes) => Type {
                id: parse_quote! {Vec<u8>},
                parent_path: empty_type_path(),
                type_desc: TypeDesc::Bytes,
            },
            RefOrType::Type(DiscoTypeDesc::Date) => Type {
                id: parse_quote! {Date<chrono::UTC>},
                parent_path: parse_quote! {::chrono},
                type_desc: TypeDesc::Date,
            },
            RefOrType::Type(DiscoTypeDesc::DateTime) => Type {
                id: parse_quote! {DateTime<chrono::UTC>},
                parent_path: parse_quote! {::chrono},
                type_desc: TypeDesc::DateTime,
            },
            RefOrType::Type(DiscoTypeDesc::Enumeration(enums)) => {
                use discovery_parser::EnumDesc as DiscoEnumDesc;
                Type {
                    id: parse_quote! {#ident},
                    parent_path: parent_path.clone(),
                    type_desc: TypeDesc::Enum(
                        enums
                            .iter()
                            .map(|DiscoEnumDesc { description, value }| {
                                let ident = to_ident(&to_rust_typestr(&value));
                                EnumDesc {
                                    ident,
                                    description: Some(description.clone()),
                                }
                            })
                            .collect(),
                    ),
                }
            }
            RefOrType::Type(DiscoTypeDesc::Array { items }) => {
                let items_ident = to_ident(&to_rust_typestr(&format!("{}-items", ident)));
                let item_type =
                    Type::from_disco_ref_or_type(&items_ident, &parent_path, &items, all_schemas);
                let item_path = item_type.type_path();
                Type {
                    id: parse_quote! {Vec<#item_path>},
                    parent_path: empty_type_path(),
                    type_desc: TypeDesc::Array {
                        items: Box::new(item_type),
                    },
                }
            }
            RefOrType::Type(DiscoTypeDesc::Object {
                properties,
                additional_properties,
            }) => {
                use discovery_parser::PropertyDesc as DiscoPropDesc;
                let props = properties
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

                let add_props = additional_properties.as_ref().map(|prop_desc| {
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
                Type {
                    id: parse_quote! {#ident},
                    parent_path: parent_path.clone(),
                    type_desc: TypeDesc::Object { props, add_props },
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
        match &self.type_desc {
            TypeDesc::Enum(enums) => {
                let variants = enums
                    .iter()
                    .map(|EnumDesc { description, ident }| {
                        parse_quote! {
                            #[doc = #description]
                            #ident
                        }
                    })
                    .collect();
                Some(TypeDef {
                    id: self.id.clone(),
                    parent_path: self.parent_path.clone(),
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
                            parse_quote! {HashMap<String, #add_props_type_path},
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
                        typ: TypeFields::Struct { fields },
                    })
                }
                (true, Some(_)) => None,
                (true, None) => panic!("object without properties or additional_properties"),
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
        props: HashMap<syn::Ident, PropertyDesc>,
        add_props: Option<Box<PropertyDesc>>,
    },
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
    typ: TypeFields,            // definition of this type
}

impl quote::ToTokens for TypeDef {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        use quote::TokenStreamExt;
        let name = &self.id;
        tokens.append_all(std::iter::once(match &self.typ {
            TypeFields::Enum { variants } => {
                quote! {
                    pub enum #name {
                        #(#variants,)*
                    }
                }
            }
            TypeFields::Struct { fields } => {
                quote! {
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
