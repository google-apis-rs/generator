use std::{ffi::OsString, path::PathBuf};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
pub struct Args {
    /// The mapped index with information about APIs. It should correspond to the Cargo workspace file.
    #[structopt(parse(from_os_str))]
    pub index_path: PathBuf,

    /// The path to the cargo.toml file defining the workspace for all generated API code
    #[structopt(parse(from_os_str))]
    pub cargo_manifest_path: PathBuf,

    /// The directory into which we will wrote the generated APIs for dumping error information
    #[structopt(parse(from_os_str))]
    pub output_directory: PathBuf,

    /// All arguments to be provided to cargo
    #[structopt(parse(from_os_str))]
    #[structopt(raw(min_values = "1"))]
    pub cargo_arguments: Vec<OsString>,
}
