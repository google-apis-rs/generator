use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
pub struct Args {
    /// If set, we will use the given original, unmapped index file instead.
    /// Useful if you want to fetch all available APIs, not only the ones we know work.
    /// Note this is a bit funky just because that makes it easier to control as _addition_
    /// from 'make'
    #[structopt(long = "use-original-index-at")]
    #[structopt(parse(from_os_str))]
    pub original_index_path: Option<PathBuf>,
    /// The mapped index with all API specification URLs as provided by Google's discovery API
    #[structopt(parse(from_os_str))]
    pub mapped_index_path: PathBuf,

    /// The directory into which we will write all downloaded specifications
    #[structopt(parse(from_os_str))]
    pub output_directory: PathBuf,
}
