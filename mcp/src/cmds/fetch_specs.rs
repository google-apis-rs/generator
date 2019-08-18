use crate::options::fetch_specs::Args;
use failure::Error;

pub fn execute(
    Args {
        discovery_json_path: _,
        output_directory: _,
    }: Args,
) -> Result<(), Error> {
    unimplemented!("fetch specs")
}
