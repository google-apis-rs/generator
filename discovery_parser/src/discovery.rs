// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::[object Object];
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: [object Object] = serde_json::from_str(&json).unwrap();
// }

#[derive(Serialize, Deserialize)]
pub struct ApiIndexV1 {
    #[serde(rename = "kind")]
    pub kind: String,

    #[serde(rename = "discoveryVersion")]
    pub discovery_version: String,

    #[serde(rename = "items")]
    pub items: Vec<Item>,
}

#[derive(Serialize, Deserialize)]
pub struct Item {
    #[serde(rename = "kind")]
    pub kind: Kind,

    #[serde(rename = "id")]
    pub id: String,

    #[serde(rename = "name")]
    pub name: String,

    #[serde(rename = "version")]
    pub version: String,

    #[serde(rename = "title")]
    pub title: String,

    #[serde(rename = "description")]
    pub description: String,

    #[serde(rename = "discoveryRestUrl")]
    pub discovery_rest_url: String,

    #[serde(rename = "icons")]
    pub icons: Icons,

    #[serde(rename = "documentationLink")]
    pub documentation_link: Option<String>,

    #[serde(rename = "preferred")]
    pub preferred: bool,

    #[serde(rename = "discoveryLink")]
    pub discovery_link: Option<String>,

    #[serde(rename = "labels")]
    pub labels: Option<Vec<Label>>,
}

#[derive(Serialize, Deserialize)]
pub struct Icons {
    #[serde(rename = "x16")]
    pub x16: String,

    #[serde(rename = "x32")]
    pub x32: String,
}

#[derive(Serialize, Deserialize)]
pub enum Kind {
    #[serde(rename = "discovery#directoryItem")]
    DiscoveryDirectoryItem,
}

#[derive(Serialize, Deserialize)]
pub enum Label {
    #[serde(rename = "labs")]
    Labs,

    #[serde(rename = "limited_availability")]
    LimitedAvailability,
}
