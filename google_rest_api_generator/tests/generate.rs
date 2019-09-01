use discovery_parser::DiscoveryRestDesc;
use google_rest_api_generator as lib;
use ci_info;
use serde_json;
use shared;
use simple_logger;
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

static SPEC: &str = include_str!("./spec.json");

#[test]
fn valid_code_is_produced_for_complex_spec() -> Result<(), Box<dyn Error>> {
    // On CI, we run more thorough integration tests at the end, which includes
    // cargo check and cargo doc
    if ci_info::is_ci() {
        return Ok(())
    }
    simple_logger::init_with_level("INFO".parse()?)?;
    let spec: DiscoveryRestDesc = serde_json::from_str(SPEC)?;
    let temp_dir = TempDir::new_in(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/output"))?;
    lib::generate("generated-api", &spec, &temp_dir)?;

    let standard = shared::Standard::default();
    let lib_path = temp_dir.path().join(&standard.lib_dir);

    fixup(&lib_path, &standard)?;

    let status = cargo(&lib_path, "check")?;
    assert!(status.success(), "cargo check failed on library");

    Ok(())
}

fn cargo(current_dir: &Path, sub_command: &str) -> Result<ExitStatus, io::Error> {
    Command::new("cargo")
        .arg(sub_command)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(current_dir)
        .status()
}

fn fixup(path: &Path, standard: &shared::Standard) -> Result<(), Box<dyn Error>> {
    let cargo_toml_path = path.join(&standard.cargo_toml_path);
    let toml = std::fs::read_to_string(&cargo_toml_path)?;
    let mut document = toml_edit::Document::from_str(&toml)?;

    document["workspace"] = toml_edit::table();
    let dependencies = &mut document["dependencies"];
    for name in &["google_field_selector", "google_api_auth"] {
        let dep = dependencies[name].as_inline_table_mut().expect(name);

        dep.remove("git");
        dep.get_or_insert("path", format!("../../../../../{}", name));
    }

    std::fs::write(&cargo_toml_path, &document.to_string().as_bytes())?;
    Ok(())
}
