const CARGO_TOML: &str = r#"
[package]
name = "{crate_name}"
version = "{crate_version}"
authors = ["Sebastian Thiel <byronimo@gmail.com>"]
edition = "2018"
# for now, let's not even accidentally publish these
publish = false

[[bin]]
name = "{bin_name}"
path = "{bin_path}"

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

pub(crate) fn cargo_toml(api: &shared::Api, standard: &shared::Standard) -> String {
    let mut doc = CARGO_TOML
        .trim()
        .replace("{crate_name}", &api.cli_crate_name)
        .replace("{crate_version}", &standard.lib_crate_version)
        .replace("{bin_name}", &api.bin_name)
        .replace("{bin_path}", &standard.main_path);

    doc.push_str(&format!("\n[dependencies.{}]\n", api.lib_crate_name));
    doc.push_str(&format!("version = \"{}\"", standard.lib_crate_version));

    doc
}
