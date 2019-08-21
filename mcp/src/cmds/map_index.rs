use super::util::logged_write;
use crate::options::map_index::Args;
use discovery_parser::generated::ApiIndexV1;
use failure::{format_err, Error, ResultExt};
use shared::MappedIndex;
use std::{convert::TryFrom, fs};

pub fn execute(
    Args {
        discovery_json_path,
        output_file,
        spec_directory,
        output_directory,
    }: Args,
) -> Result<(), Error> {
    let index: ApiIndexV1 = { serde_json::from_slice(&fs::read(&discovery_json_path)?) }
        .with_context(|_| {
            format_err!(
                "Could read spec file at '{}'",
                discovery_json_path.display()
            )
        })?;

    let index: MappedIndex =
        MappedIndex::try_from(index)?.validated(&spec_directory, &output_directory);
    logged_write(
        output_file,
        serde_json::to_string_pretty(&index)?.as_bytes(),
        "mapped api index",
    )
}
