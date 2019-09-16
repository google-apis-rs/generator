use discovery_parser::DiscoveryRestDesc;
use google_cli_generator as lib;
use serde_json;
use shared;
use simple_logger;
use std::convert::TryFrom;
use std::{
    error::Error,
    io,
    path::Path,
    process::Stdio,
    process::{Command, ExitStatus},
    str::FromStr,
};
use tempfile::TempDir;
use toml_edit;

static SPEC: &str = include_str!("spec.json");

#[test]
fn valid_code_is_produced_for_complex_spec() -> Result<(), Box<dyn Error>> {
    simple_logger::init_with_level("INFO".parse()?)?;
    let spec: DiscoveryRestDesc = serde_json::from_str(SPEC)?;
    let temp_dir = TempDir::new_in(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/output"))?;
    lib::all::generate(&spec, &temp_dir, lib::all::Build::ApiAndCli)?;

    let standard = shared::Standard::default();
    let lib_path = temp_dir.path().join(&standard.lib_dir);
    let cli_path = temp_dir.path().join(&standard.cli_dir);

    let api = shared::Api::try_from(&spec)?;
    fixup_deps(&lib_path, &standard)?;
    fixup_deps(&cli_path, &standard)?;
    fixup_cli(&cli_path, &api, &standard)?;

    let status = cargo(&cli_path, "check")?;
    assert!(status.success(), "cargo check failed on library");

    Ok(())
}

fn cargo(current_dir: &Path, sub_command: &str) -> Result<ExitStatus, io::Error> {
    let mut cmd = Command::new("cargo");
    cmd.arg(sub_command)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(current_dir);

    if std::env::var("TRAIN_MODE").is_ok() {
        cmd.arg("--offline");
    }

    cmd.status()
}

fn fixup_cli(
    path: &Path,
    api: &shared::Api,
    standard: &shared::Standard,
) -> Result<(), Box<dyn Error>> {
    let (cargo_toml_path, mut document) = toml_document(path, &standard)?;

    document["dependencies"][&api.lib_crate_name]["path"] = toml_edit::value(
        Path::new("..")
            .join(&standard.lib_dir)
            .to_str()
            .expect("valid utf8"),
    );

    std::fs::write(&cargo_toml_path, &document.to_string().as_bytes())?;
    Ok(())
}

fn fixup_deps(path: &Path, standard: &shared::Standard) -> Result<(), Box<dyn Error>> {
    let (cargo_toml_path, mut document) = toml_document(path, &standard)?;

    document["workspace"] = toml_edit::table();
    let dependencies = &mut document["dependencies"];
    for name in &[
        "google_field_selector",
        "google_api_auth",
        "google_api_bytes",
        "google_cli_shared",
    ] {
        let dep = dependencies[name].as_inline_table_mut();
        if let Some(dep) = dep {
            dep.remove("git");
            dep.get_or_insert("path", format!("../../../../../{}", name));
        }
    }

    std::fs::write(&cargo_toml_path, &document.to_string().as_bytes())?;
    Ok(())
}

fn toml_document(
    path: &Path,
    standard: &shared::Standard,
) -> Result<(std::path::PathBuf, toml_edit::Document), Box<dyn Error>> {
    let cargo_toml_path = path.join(&standard.cargo_toml_path);
    let toml = std::fs::read_to_string(&cargo_toml_path)?;
    let document = toml_edit::Document::from_str(&toml)?;

    Ok((cargo_toml_path, document))
}
