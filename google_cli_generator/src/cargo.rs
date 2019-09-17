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
clap = "^2.33"
serde_json = "1.0.40"
dirs = "2.0"
google_cli_shared = { git = "https://github.com/google-apis-rs/generator", version = "0.1.0" }
default-boxed = "0.1.6"
"#;

pub(crate) fn cargo_toml(api: &shared::Api, standard: &shared::Standard) -> String {
    let mut doc = CARGO_TOML
        .trim()
        .replace("{crate_name}", &api.cli_crate_name)
        .replace(
            "{crate_version}",
            &api.cli_crate_version
                .as_ref()
                .expect("available crate version"),
        )
        .replace("{bin_name}", &api.bin_name)
        .replace("{bin_path}", &standard.main_path);

    doc.push_str(&format!("\n[dependencies.{}]\n", api.lib_crate_name));
    doc.push_str(&format!("path = \"../lib\"\n"));
    doc.push_str(&format!(
        "version = \"{}\"\n",
        api.lib_crate_version
            .as_ref()
            .expect("available crate version")
    ));

    doc
}
