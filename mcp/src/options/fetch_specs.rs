use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
pub struct Args {
    /// The mapped index with all API specification URLs as provided by Google's discovery API
    #[structopt(parse(from_os_str))]
    pub mapped_index_path: PathBuf,

    /// The directory into which we will write all downloaded specifications
    #[structopt(parse(from_os_str))]
    pub output_directory: PathBuf,
}
