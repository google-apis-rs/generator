const CARGO_TOML: &str = r#"
[package]
name = "{crate_name}"
version = "{crate_version}"
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
) -> String {
    let mut doc = CARGO_TOML
        .trim()
        .replace("{crate_name}", crate_name)
        .replace("{crate_version}", &standard.lib_crate_version);

    if include_bytes_dep {
        doc.push_str("\n[dependencies.google_api_bytes]\n");
        doc.push_str("git = \"https://github.com/google-apis-rs/generator\"\n");
    }
    doc
}
