use toml_edit::{value, Document};

const CARGO_TOML: &str = r#"
[package]
name = "CRATE NAME GOES HERE"
version = "VERSION GOES HERE"
authors = ["Glenn Griffin <ggriffiniii@gmail.com"]
edition = "2018"
# for now, let's not even accidentally publish these
publish = false

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
reqwest = "0.9"
google_field_selector = { git = "https://github.com/google-apis-rs/generator" }
mime = "0.3"
textnonce = "0.6"
yup-oauth2 = "3"
tokio = "0.1"
percent-encoding = "2"
radix64 = "0.6"
"#;

pub(crate) fn cargo_toml(crate_name: impl Into<String>) -> Document {
    let mut doc: Document = CARGO_TOML.trim().parse().unwrap();
    let package = &mut doc["package"];
    package["name"] = value(crate_name.into());
    package["version"] = value("0.1.0");
    doc
}
