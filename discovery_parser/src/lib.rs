use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap;

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
#[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
pub struct DiscoveryRestDesc {
    pub kind: Option<String>,
    pub etag: Option<String>,
    pub discovery_version: Option<String>,
    pub id: String,
    pub name: String,
    pub canonical_name: Option<String>,
    pub fully_encode_reserved_expansion: Option<bool>,
    pub version: String,
    pub revision: String,
    pub title: String,
    pub description: String,
    pub owner_domain: String,
    pub owner_name: String,
    #[serde(default)]
    pub icons: BTreeMap<String, String>,
    pub documentation_link: Option<String>,
    pub protocol: String,
    pub base_url: String,
    pub base_path: String,
    pub root_url: String,
    pub service_path: String,
    pub batch_path: String,
    #[serde(rename="version_module")]
    pub version_module: Option<bool>,
    pub package_path: Option<String>,
    pub labels: Option<Vec<String>>,
    pub features: Option<Vec<String>>,
    #[serde(default)]
    pub parameters: BTreeMap<String, ParamDesc>,
    pub auth: Option<AuthDesc>,
    #[serde(default)]
    pub schemas: BTreeMap<String, SchemaDesc>,
    pub resources: BTreeMap<String, ResourceDesc>,
    #[serde(default)]
    pub methods: BTreeMap<String, MethodDesc>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
#[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
pub struct ParamDesc {
    pub description: Option<String>,
    pub default: Option<String>,
    pub location: String,
    #[serde(default)]
    pub required: bool,
    #[serde(rename = "type")]
    pub typ: String,
    pub format: Option<String>,
    pub minimum: Option<String>,
    pub maximum: Option<String>,
    pub pattern: Option<String>,
    #[serde(default, rename = "enum")]
    pub enumeration: Vec<String>,
    #[serde(default)]
    pub enum_descriptions: Vec<String>,
    #[serde(default)]
    pub repeated: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
pub struct AuthDesc {
    pub oauth2: Oauth2Desc,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
pub struct Oauth2Desc {
    #[serde(default)]
    pub scopes: BTreeMap<String, ScopeDesc>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
pub struct ScopeDesc {
    pub description: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "strict", serde(deny_unknown_fields))]
pub struct SchemaDesc {
    pub id: String,
    pub description: Option<String>,
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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeDesc {
    #[serde(rename = "type")]
    pub typ: String,
    pub format: Option<String>,
    #[serde(default, rename = "enum")]
    pub enumeration: Vec<String>,
    #[serde(default)]
    pub enum_descriptions: Vec<String>,
    #[serde(default)]
    pub properties: BTreeMap<String, PropertyDesc>,
    #[serde(default)]
    pub additional_properties: Option<Box<PropertyDesc>>,
    pub items: Option<Box<RefOrType<TypeDesc>>>,
}

impl TypeDesc {
    pub fn from_param(param: ParamDesc) -> TypeDesc {
        TypeDesc {
            typ: param.typ,
            format: param.format,
            enumeration: param.enumeration,
            enum_descriptions: param.enum_descriptions,
            properties: BTreeMap::new(),
            additional_properties: None,
            items: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDesc {
    #[serde(default)]
    pub resources: BTreeMap<String, ResourceDesc>,
    #[serde(default)]
    pub methods: BTreeMap<String, MethodDesc>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MethodDesc {
    pub id: String,
    pub path: String,
    pub http_method: String,
    pub description: Option<String>,
    #[serde(default)]
    pub parameters: BTreeMap<String, ParamDesc>,
    #[serde(default)]
    pub parameter_order: Vec<String>,
    pub request: Option<RefOrType<TypeDesc>>,
    pub response: Option<RefOrType<TypeDesc>>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub supports_media_download: bool,
    #[serde(default)]
    pub use_media_download_service: bool,
    #[serde(default)]
    pub supports_subscription: bool,
    #[serde(default)]
    pub supports_media_upload: bool,
    pub media_upload: Option<MediaUpload>,
}


#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaUpload {
    accept: Vec<String>,
    max_size: String,
    protocols: UploadProtocols,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadProtocols {
    pub simple: Option<UploadProtocol>,
    pub resumable: Option<UploadProtocol>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadProtocol {
    multipart: bool,
    path: String,
}

