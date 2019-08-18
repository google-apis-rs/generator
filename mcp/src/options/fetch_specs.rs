use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "fetch-specs", about = "Fetch all API specs, in parallel")]
pub struct Args {
    /// The index with all API specification URLs as provided by Google's discovery API
    #[structopt(parse(from_os_str))]
    discovery_json_path: PathBuf,

    /// The directory into which we will write all downloaded specifications
    #[structopt(parse(from_os_str))]
    output_directory: PathBuf,
}
