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
google_api_auth = { git = "https://github.com/google-apis-rs/generator" }
mime = "0.3"
textnonce = "0.6"
percent-encoding = "2"
"#;

pub(crate) fn cargo_toml(
    crate_name: &str,
    include_bytes_dep: bool,
    standard: &shared::Standard,
) -> Document {
    let mut doc: Document = CARGO_TOML.trim().parse().unwrap();

    let package = &mut doc["package"];
    package["name"] = value(crate_name);
    package["version"] = value(standard.lib_crate_version.as_str());

    if include_bytes_dep {
        doc["dependencies"]["google_api_bytes"]["git"] =
            value("https://github.com/google-apis-rs/generator");
    }
    doc
}
