use super::CombinedMetadata;
use crate::cli;
use discovery_parser::DiscoveryRestDesc;
use google_rest_api_generator::generate as generate_library;
use std::{error::Error, path::Path};

pub enum Build {
    ApiAndCliInParallelNoErrorHandling,
    ApiAndCli,
    OnlyCli,
    OnlyApi,
}

pub fn generate(
    discovery_desc: &DiscoveryRestDesc,
    base_dir: impl AsRef<Path>,
    mode: Build,
) -> Result<(), Box<dyn Error>> {
    let constants = shared::Standard::default();
    std::fs::write(
        base_dir.as_ref().join(constants.metadata_path),
        serde_json::to_string_pretty(&CombinedMetadata::default())?,
    )?;

    let lib_dir = base_dir.as_ref().join(&constants.lib_dir);
    let cli_dir = base_dir.as_ref().join(&constants.cli_dir);
    use self::Build::*;
    match mode {
        ApiAndCliInParallelNoErrorHandling => {
            let _ignore_errors_while_cli_gen_may_fail = crossbeam::scope(|s| {
                s.spawn(|_| generate_library(lib_dir, &discovery_desc).map_err(|e| e.to_string()));
                s.spawn(|_| cli::generate(cli_dir, &discovery_desc).map_err(|e| e.to_string()));
            });
        }
        ApiAndCli => {
            generate_library(lib_dir, &discovery_desc)?;
            cli::generate(cli_dir, &discovery_desc)?;
        }
        OnlyCli => cli::generate(cli_dir, &discovery_desc)?,
        OnlyApi => generate_library(lib_dir, &discovery_desc)?,
    }

    Ok(())
}
