use std::path::Path;
use toml_edit::{value, Document};

const CARGO_TOML_LIB: &str = r#"
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

const CARGO_TOML_CLI: &str = r#"
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

[dependencies.google-urlshortener1]
path = "../gen/urlshortener/v1/lib"
version = "0.1.0"

[dependencies.google-cli-shared]
path = "google_cli_shared"
git = "https://github.com/google-apis-rs/generator"
version = "0.1.0"

[workspace]
"#;

pub(crate) fn cargo_toml_lib(
    crate_name: impl Into<String>,
    include_bytes_dep: bool,
    standard: &shared::Standard,
) -> Document {
    let mut doc: Document = CARGO_TOML_LIB.trim().parse().unwrap();
    fill_common_fields(crate_name, &mut doc, &standard.lib_crate_version);
    if include_bytes_dep {
        doc["dependencies"]["google_api_bytes"]["git"] =
            value("https://github.com/google-apis-rs/generator");
    }
    doc
}

fn fill_common_fields(crate_name: impl Into<String>, doc: &mut Document, crate_version: &str) {
    let package = &mut doc["package"];
    package["name"] = value(crate_name.into());
    package["version"] = value(crate_version);
}

pub(crate) fn cargo_toml_cli(
    api: &shared::Api,
    lib_dir_from_cli_dir_path: impl AsRef<Path>,
    standard: &shared::Standard,
) -> Document {
    let mut doc: Document = CARGO_TOML_CLI.trim().parse().unwrap();
    fill_common_fields(&api.cli_crate_name, &mut doc, &standard.cli_crate_version);

    dbg!(&doc["bin"]);
    let bin = doc["bin"]
        .as_array_of_tables_mut()
        .expect("[[bin]] present")
        .get_mut(0)
        .expect("first binary is defined");
    bin["name"] = value(api.bin_name.as_str());
    bin["path"] = value(standard.main_path.as_str());

    let lib_dependency = &mut doc["dependencies"][&api.lib_crate_name];
    lib_dependency["path"] = value(
        lib_dir_from_cli_dir_path
            .as_ref()
            .to_str()
            .expect("valid utf-8"),
    );
    lib_dependency["version"] = value(standard.lib_crate_version.as_str());

    doc
}
