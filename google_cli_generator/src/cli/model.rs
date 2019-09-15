use serde::Serialize;
use shared::Api;
use discovery_parser::DiscoveryRestDesc;

#[derive(Serialize)]
pub struct Model {
    /// The name of the crate for 'use ' statement
    lib_crate_name_for_use: String,
    /// The name of the CLI program
    program_name: String,
    /// The full semantic version of the CLI
    cli_version: String,
    /// A one-line summary of what the API does
    description: String
}

impl Model {
    pub fn new(api: Api, desc: &DiscoveryRestDesc) -> Self {
        Model {
            lib_crate_name_for_use: api.lib_crate_name.replace('-', "_"),
            program_name: api.bin_name,
            cli_version: api.cli_crate_version.expect("available cli crate version"),
            description: desc.description.clone()
        }
    }
}
