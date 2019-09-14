use discovery_parser::DiscoveryRestDesc;
use google_rest_api_generator::APIDesc;
use log::info;
use std::{convert::TryFrom, error::Error, io::Write, path::Path};

use super::cargo;

pub fn generate(
    base_dir: impl AsRef<Path>,
    discovery_desc: &DiscoveryRestDesc,
) -> Result<(), Box<dyn Error>> {
    const MAIN_RS: &str = r#"
   fn main() {
    println!("Hello, world!");
   }"#;
    info!("cli: building api desc");
    let _api_desc = APIDesc::from_discovery(discovery_desc);
    let api = shared::Api::try_from(discovery_desc)?;

    let constants = shared::Standard::default();
    let base_dir = base_dir.as_ref();
    let cargo_toml_path = base_dir.join(&constants.cargo_toml_path);
    let main_path = base_dir.join(&constants.main_path);

    info!("cli: creating source directory and Cargo.toml");
    std::fs::create_dir_all(&main_path.parent().expect("file in directory"))?;

    let cargo_contents = cargo::cargo_toml(&api, &constants);
    std::fs::write(&cargo_toml_path, &cargo_contents)?;

    info!("cli: writing main '{}'", main_path.display());
    let output_file = std::fs::File::create(&main_path)?;
    let mut rustfmt_writer = shared::RustFmtWriter::new(output_file)?;
    rustfmt_writer.write_all(MAIN_RS.as_bytes())?;

    Ok(())
}
