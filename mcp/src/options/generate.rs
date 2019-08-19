use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
/// Generate libraries and command-line interface for said libraries based on Google API specifications.
///
/// The output will always be formatted using rustfmt. You can set the RUSTFMT environment variable to an
/// empty value to prevent formatting, which can safe some time during generation.
pub struct Args {
    /// The Google API specification as downloaded from the discovery service
    #[structopt(parse(from_os_str))]
    pub spec_json_path: PathBuf,

    /// The directory into which we will write all generated data
    #[structopt(parse(from_os_str))]
    pub output_directory: PathBuf,
}
