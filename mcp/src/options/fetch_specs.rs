use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
pub struct Args {
    /// Either the original Google index, or the mapped index we produced prior
    #[structopt(parse(from_os_str))]
    pub index_path: PathBuf,

    /// The directory into which we will write all downloaded specifications
    #[structopt(parse(from_os_str))]
    pub spec_directory: PathBuf,

    /// The directory into which we will write the generated APIs
    #[structopt(parse(from_os_str))]
    pub output_directory: PathBuf,
}
