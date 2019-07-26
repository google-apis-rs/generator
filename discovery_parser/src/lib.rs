use serde::{Deserialize, Deserializer};
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryError {
    pub error: ErrorMsg,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorMsg {
    pub code: u32,
    pub message: String,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum RestDescOrErr {
    RestDesc(DiscoveryRestDesc),
    Err(DiscoveryError),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryRestDesc {
    pub kind: Option<String>,
    pub etag: Option<String>,
    pub discovery_version: Option<String>,
    pub id: String,
    pub name: String,
    pub version: String,
    pub revision: String,
    pub title: String,
    pub description: String,
    pub owner_domain: String,
    pub owner_name: String,
    #[serde(default)]
    pub icons: HashMap<String, String>,
    pub documentation_link: Option<String>,
    pub protocol: String,
    pub base_url: String,
    pub base_path: String,
    pub root_url: String,
    pub service_path: String,
    pub batch_path: String,
    #[serde(default)]
    pub parameters: HashMap<String, ParamDesc>,
    pub auth: Option<AuthDesc>,
    #[serde(default)]
    pub schemas: HashMap<String, SchemaDesc>,
    pub resources: HashMap<String, ResourceDesc>,
}

#[derive(Clone, Debug)]
pub struct ParamDesc {
    pub description: Option<String>,
    pub default: Option<String>,
    pub location: String,
    pub required: bool,
    pub typ: ParamTypeDesc,
}

#[derive(Clone, Debug)]
pub struct EnumDesc {
    pub description: String,
    pub value: String,
}

#[derive(Clone, Debug)]
pub enum ParamTypeDesc {
    Boolean,
    Int32 { min: Option<i32>, max: Option<i32> },
    Uint32 { min: Option<u32>, max: Option<u32> },
    Float64 { min: Option<f64>, max: Option<f64> },
    Float32 { min: Option<f32>, max: Option<f32> },
    String { pattern: Option<String> },
    Enumeration(Vec<EnumDesc>),
    Bytes,
    Date,
    DateTime,
    Int64 { min: Option<i64>, max: Option<i64> },
    Uint64 { min: Option<u64>, max: Option<u64> },
}

impl ParamTypeDesc {
    pub fn as_type_desc(&self) -> TypeDesc {
        match self {
            ParamTypeDesc::Boolean => TypeDesc::Boolean,
            ParamTypeDesc::Int32{..} => TypeDesc::Int32,
            ParamTypeDesc::Uint32{..} => TypeDesc::Uint32,
            ParamTypeDesc::Float32{..} => TypeDesc::Float32,
            ParamTypeDesc::Float64{..} => TypeDesc::Float64,
            ParamTypeDesc::String{..} => TypeDesc::String,
            ParamTypeDesc::Enumeration(enums) => TypeDesc::Enumeration(enums.clone()),
            ParamTypeDesc::Bytes => TypeDesc::Bytes,
            ParamTypeDesc::Date => TypeDesc::Date,
            ParamTypeDesc::DateTime => TypeDesc::DateTime,
            ParamTypeDesc::Int64{..} => TypeDesc::Int64,
            ParamTypeDesc::Uint64{..} => TypeDesc::Uint64,
        }
    }
}

impl<'de> Deserialize<'de> for ParamDesc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase", tag = "type")]
        struct RawParamDesc {
            description: Option<String>,
            default: Option<String>,
            location: String,
            #[serde(default)]
            required: bool,
            #[serde(rename = "type")]
            typ: String,
            format: Option<String>,
            minimum: Option<String>,
            maximum: Option<String>,
            pattern: Option<String>,
            #[serde(default, rename = "enum")]
            enumeration: Vec<String>,
            #[serde(default)]
            enum_descriptions: Vec<String>,
        }

        let rpd = RawParamDesc::deserialize(deserializer)?;
        Ok(ParamDesc {
            description: rpd.description,
            default: rpd.default,
            location: rpd.location,
            required: rpd.required,
            typ: match (rpd.typ.as_str(), rpd.format.as_ref().map(|x| x.as_str())) {
                ("boolean", _) => ParamTypeDesc::Boolean,
                ("string", Some("byte")) => ParamTypeDesc::Bytes,
                ("string", Some("date")) => ParamTypeDesc::Date,
                ("string", Some("date-time")) => ParamTypeDesc::DateTime,
                ("string", Some("int64")) => ParamTypeDesc::Int64 {
                    min: rpd.minimum.and_then(|x| x.parse().ok()),
                    max: rpd.maximum.and_then(|x| x.parse().ok()),
                },
                ("string", Some("uint64")) => ParamTypeDesc::Uint64 {
                    min: rpd.minimum.and_then(|x| x.parse().ok()),
                    max: rpd.maximum.and_then(|x| x.parse().ok()),
                },
                ("string", _) => {
                    if rpd.enumeration.is_empty() {
                        ParamTypeDesc::String {
                            pattern: rpd.pattern,
                        }
                    } else {
                        ParamTypeDesc::Enumeration(
                            rpd.enumeration
                                .into_iter()
                                .zip(rpd.enum_descriptions.into_iter())
                                .map(|(value, description)| EnumDesc { value, description })
                                .collect(),
                        )
                    }
                }
                ("integer", Some("uint32")) => ParamTypeDesc::Uint32 {
                    min: rpd.minimum.and_then(|x| x.parse().ok()),
                    max: rpd.maximum.and_then(|x| x.parse().ok()),
                },
                ("integer", Some("int32")) => ParamTypeDesc::Int32 {
                    min: rpd.minimum.and_then(|x| x.parse().ok()),
                    max: rpd.maximum.and_then(|x| x.parse().ok()),
                },
                ("number", Some("float")) => ParamTypeDesc::Float32 {
                    min: rpd.minimum.and_then(|x| x.parse().ok()),
                    max: rpd.maximum.and_then(|x| x.parse().ok()),
                },
                ("number", Some("double")) => ParamTypeDesc::Float64 {
                    min: rpd.minimum.and_then(|x| x.parse().ok()),
                    max: rpd.maximum.and_then(|x| x.parse().ok()),
                },
                _ => return Err(D::Error::custom("Unknown param type")),
            },
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthDesc {
    pub oauth2: Oauth2Desc,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Oauth2Desc {
    #[serde(default)]
    pub scopes: HashMap<String, ScopeDesc>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeDesc {
    pub description: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaDesc {
    pub id: String,
    #[serde(flatten, rename = "type")]
    pub typ: TypeDesc,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyDesc {
    pub description: Option<String>,

    #[serde(flatten)]
    pub typ: RefOrType<TypeDesc>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum RefOrType<T> {
    #[serde(deserialize_with = "ref_target")]
    Ref(String),
    Type(T),
}

fn ref_target<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RefTarget {
        #[serde(rename = "$ref")]
        reference: String,
    }
    let rt = RefTarget::deserialize(deserializer)?;
    Ok(rt.reference)
}

#[derive(Clone, Debug)]
pub enum TypeDesc {
    Any,
    String,
    FormattedString {
        format: String,
    },
    Boolean,
    Int32,
    Uint32,
    Float64,
    Float32,
    Bytes,
    Date,
    DateTime,
    Int64,
    Uint64,
    Enumeration(Vec<EnumDesc>),
    Array {
        items: Box<RefOrType<TypeDesc>>,
    },
    Object {
        properties: HashMap<String, PropertyDesc>,
        additional_properties: Option<Box<PropertyDesc>>,
    },
}

impl<'de> Deserialize<'de> for TypeDesc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RawTypeDesc {
            #[serde(rename = "type")]
            typ: String,
            format: Option<String>,
            #[serde(default, rename = "enum")]
            enumeration: Vec<String>,
            #[serde(default)]
            enum_descriptions: Vec<String>,
            #[serde(default)]
            properties: HashMap<String, PropertyDesc>,
            #[serde(default)]
            additional_properties: Option<Box<PropertyDesc>>,
            items: Option<Box<RefOrType<TypeDesc>>>,
        };
        let rtd = RawTypeDesc::deserialize(deserializer).map_err(|x| {
            println!("{:?}", x);
            x
        })?;
        Ok(
            match (rtd.typ.as_str(), rtd.format.as_ref().map(|x| x.as_str())) {
                ("any", None) => TypeDesc::Any,
                ("string", None) => {
                    if rtd.enumeration.is_empty() {
                        TypeDesc::String
                    } else {
                        TypeDesc::Enumeration(
                            rtd.enumeration
                                .into_iter()
                                .zip(rtd.enum_descriptions)
                                .map(|(value, description)| EnumDesc { value, description })
                                .collect(),
                        )
                    }
                }
                ("boolean", None) => TypeDesc::Boolean,
                ("integer", Some("int32")) => TypeDesc::Int32,
                ("integer", Some("uint32")) => TypeDesc::Uint32,
                ("number", Some("double")) => TypeDesc::Float64,
                ("number", Some("float")) => TypeDesc::Float32,
                ("string", Some("byte")) => TypeDesc::Bytes,
                ("string", Some("date")) => TypeDesc::Date,
                ("string", Some("date-time")) => TypeDesc::DateTime,
                ("string", Some("int64")) => TypeDesc::Int64,
                ("string", Some("uint64")) => TypeDesc::Uint64,
                ("string", Some(format)) => TypeDesc::FormattedString {
                    format: format.to_owned(),
                },
                ("array", None) => {
                    if let Some(items) = rtd.items {
                        TypeDesc::Array { items }
                    } else {
                        return Err(D::Error::custom("no items specified within array"));
                    }
                }
                ("object", None) => TypeDesc::Object {
                    properties: rtd.properties,
                    additional_properties: rtd.additional_properties,
                },
                _ => return Err(D::Error::custom("Unknown type")),
            },
        )
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDesc {
    #[serde(default)]
    pub methods: HashMap<String, MethodDesc>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MethodDesc {
    pub id: String,
    pub path: String,
    pub http_method: String,
    pub description: Option<String>,
    #[serde(default)]
    pub parameters: HashMap<String, ParamDesc>,
    #[serde(default)]
    pub parameter_order: Vec<String>,
    pub request: Option<RefOrType<TypeDesc>>,
    pub response: Option<RefOrType<TypeDesc>>,
    #[serde(default)]
    pub scopes: Vec<String>,
}