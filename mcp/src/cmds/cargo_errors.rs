use crate::options::cargo_errors::Args;
use failure::Error;

pub fn execute(
    Args {
        index_path,
        cargo_manifest_path,
        output_directory,
        cargo_arguments,
    }: Args,
) -> Result<(), Error> {
    unimplemented!()
}
