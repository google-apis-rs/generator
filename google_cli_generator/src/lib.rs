use discovery_parser::DiscoveryRestDesc;
use google_rest_api_generator::{generate as generate_library, APIDesc, Metadata as ApiMetadata};
use log::info;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, error::Error, io::Write, path::Path};

mod cargo;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct Metadata {
    pub git_hash: String,
    pub ymd_date: String,
}

impl Default for Metadata {
    fn default() -> Self {
        Metadata {
            git_hash: env!("GIT_HASH").into(),
            ymd_date: env!("BUILD_DATE").into(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Default)]
pub struct CombinedMetadata {
    pub cli_generator: Metadata,
    pub api_generator: ApiMetadata,
}

pub fn generate(
    discovery_desc: &DiscoveryRestDesc,
    base_dir: impl AsRef<Path>,
) -> Result<(), Box<dyn Error>> {
    let constants = shared::Standard::default();
    std::fs::write(
        base_dir.as_ref().join(constants.metadata_path),
        serde_json::to_string_pretty(&CombinedMetadata::default())?,
    )?;

    let lib_dir = base_dir.as_ref().join(&constants.lib_dir);
    let cli_dir = base_dir.as_ref().join(&constants.cli_dir);
    crossbeam::scope(|s| {
        s.spawn(|_| generate_library(lib_dir, &discovery_desc).map_err(|e| e.to_string()));
        s.spawn(|_| generate_cli(cli_dir, &discovery_desc).map_err(|e| e.to_string()));
    })
    .unwrap();

    Ok(())
}

pub fn generate_cli(
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

    let cargo_contents = cargo::cargo_toml(&api, &constants).to_string();
    std::fs::write(&cargo_toml_path, &cargo_contents)?;

    info!("cli: writing main '{}'", main_path.display());
    let output_file = std::fs::File::create(&main_path)?;
    let mut rustfmt_writer = shared::RustFmtWriter::new(output_file)?;
    rustfmt_writer.write_all(MAIN_RS.as_bytes())?;

    Ok(())
}
