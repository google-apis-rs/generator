use crate::options::generate::Args;
use discovery_parser::DiscoveryRestDesc;
use failure::{format_err, Error, ResultExt};
use google_rest_api_generator::generate as generate_library;
use shared::{Api, Standard};
use std::{convert::TryFrom, fs};

pub fn execute(
    Args {
        spec_json_path,
        output_directory,
    }: Args,
) -> Result<(), Error> {
    let desc: DiscoveryRestDesc = { serde_json::from_slice(&fs::read(&spec_json_path)?) }
        .with_context(|_| format_err!("Could read spec file at '{}'", spec_json_path.display()))?;

    let api = Api::try_from(&desc)?;
    let standard = Standard::default();
    generate_library(
        &api.crate_name,
        &desc,
        output_directory.join(standard.lib_dir),
    )
    .map_err(|e| format_err!("{}", e.to_string()))?;
    Ok(())
}
