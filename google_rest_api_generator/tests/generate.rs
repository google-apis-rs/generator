use discovery_parser::DiscoveryRestDesc;
use google_rest_api_generator as lib;
use serde_json;
use shared;
use simple_logger;
use std::{
    io,
    path::Path,
    process::Stdio,
    process::{Command, ExitStatus},
};
use tempfile::TempDir;

static SPEC: &str = include_str!("./spec.json");

#[test]
fn valid_code_is_produced_for_complex_spec() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level("INFO".parse()?)?;
    let spec: DiscoveryRestDesc = serde_json::from_str(SPEC)?;
    let temp_dir = TempDir::new_in(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/output"))?;
    lib::generate("spec", &spec, &temp_dir)?;
    let standard = shared::Standard::default();
    let lib_path = temp_dir.path().join(standard.lib_dir);

    let status = cargo(&lib_path, "check")?;
    assert!(status.success(), "cargo check failed on library");

    let status = cargo(&lib_path, "doc")?;
    assert!(status.success(), "cargo doc failed on library");

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
