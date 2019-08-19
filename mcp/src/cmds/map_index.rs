use super::util::logged_write;
use crate::options::map_index::Args;
use discovery_parser::generated::{ApiIndexV1, Item};
use failure::{format_err, Error, ResultExt};
use serde::Serialize;
use std::{convert::TryFrom, fs};

#[derive(Serialize)]
struct MappedIndex {
    api: Vec<Api>,
}

#[derive(Serialize)]
struct Api {
    crate_name: String,
    make_target: String,
}

impl TryFrom<Item> for Api {
    type Error = Error;

    fn try_from(value: Item) -> Result<Self, Self::Error> {
        Ok(Api {
            crate_name: value.name.clone(),
            make_target: value.name.clone(),
        })
    }
}

pub fn execute(
    Args {
        discovery_json_path,
        output_file,
    }: Args,
) -> Result<(), Error> {
    let desc: ApiIndexV1 = { serde_json::from_slice(&fs::read(&discovery_json_path)?) }
        .with_context(|_| {
            format_err!(
                "Could read spec file at '{}'",
                discovery_json_path.display()
            )
        })?;

    let output = MappedIndex {
        api: desc
            .items
            .into_iter()
            .map(Api::try_from)
            .collect::<Result<Vec<_>, Error>>()?,
    };
    logged_write(
        output_file,
        serde_json::to_string_pretty(&output)?.as_bytes(),
        "mapped api index",
    )
}
