use crate::options::generate::Args;
use discovery_parser::DiscoveryRestDesc;
use failure::{format_err, Error, ResultExt};
use google_rest_api_generator::generate as generate_library;
use std::fs;

pub fn execute(
    Args {
        spec_json_path,
        output_directory,
    }: Args,
) -> Result<(), Error> {
    let desc: DiscoveryRestDesc = { serde_json::from_slice(&fs::read(&spec_json_path)?) }
        .with_context(|_| format_err!("Could read spec file at '{}'", spec_json_path.display()))?;

    // TODO: I vote for making the crate names compatible to the existing ones, and signal breakage via semver
    let project_name = format!("google_{}_{}", &desc.name, &desc.version);
    generate_library(&project_name, &desc, output_directory.join("lib"))
        .map_err(|e| format_err!("{}", e.to_string()))?;
    Ok(())
}
