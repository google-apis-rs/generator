use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(setting = structopt::clap::AppSettings::ColoredHelp)]
/// Transform the Google API index into something we can use further when dealing with substitutions.
pub struct Args {
    /// The index with all API specification URLs as provided by Google's discovery API
    #[structopt(parse(from_os_str))]
    pub discovery_json_path: PathBuf,

    /// The path to which to write the digest
    #[structopt(parse(from_os_str))]
    pub output_file: PathBuf,

    /// The directory into which the `fetch-specs` subcommand writes its files, see `Standard::spec_dir`
    #[structopt(parse(from_os_str))]
    pub spec_directory: PathBuf,

    /// The directory into which files will be generated into
    #[structopt(parse(from_os_str))]
    pub output_directory: PathBuf,
}
