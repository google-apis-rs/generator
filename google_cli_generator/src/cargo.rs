use toml_edit::{value, Document};

const CARGO_TOML: &str = r#"
[package]
name = "CRATE NAME GOES HERE"
version = "VERSION GOES HERE"
authors = ["Sebastian Thiel <byronimo@gmail.com>"]
edition = "2018"
# for now, let's not even accidentally publish these
publish = false

[[bin]]
name = "BIN NAME GOES HERE"
path = "MAIN SOURCE FILE PATH GOES HERE"

[dependencies]
yup-oauth2 = { git = "https://github.com/dermesser/yup-oauth2", rev = "778e5af" } # Use released version once it includes this commit
google_api_auth = { git = "https://github.com/google-apis-rs/generator", features = ["with-yup-oauth2"] }
hyper-rustls = "^0.16"
clap = "^2.33"
hyper = "0.12.33"
serde_json = "1.0.40"
google_cli_shared = { git = "https://github.com/google-apis-rs/generator", version = "0.1.0" }

[workspace]
"#;

pub(crate) fn cargo_toml(api: &shared::Api, standard: &shared::Standard) -> Document {
    let mut doc: Document = CARGO_TOML.trim().parse().unwrap();

    let package = &mut doc["package"];
    package["name"] = value(api.cli_crate_name.as_str());
    package["version"] = value(standard.cli_crate_version.as_str());

    let bin = doc["bin"]
        .as_array_of_tables_mut()
        .expect("[[bin]] present")
        .get_mut(0)
        .expect("first binary is defined");
    bin["name"] = value(api.bin_name.as_str());
    bin["path"] = value(standard.main_path.as_str());

    let lib_dependency = &mut doc["dependencies"][&api.lib_crate_name];
    lib_dependency["version"] = value(standard.lib_crate_version.as_str());

    doc
}
