use super::util::logged_write;
use crate::options::map_index::Args;
use crate::shared::{crate_name, make_target, sanitized_name};
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
    name: String,
    crate_name: String,
    make_target: String,
    #[serde(skip)]
    original: Item,
}

impl TryFrom<Item> for Api {
    type Error = Error;

    fn try_from(value: Item) -> Result<Self, Self::Error> {
        Ok(Api {
            name: sanitized_name(&value.name).into(),
            crate_name: crate_name(&value.name, &value.version)?,
            make_target: make_target(&value.name, &value.version)?,
            original: value,
        })
    }
}

impl MappedIndex {
    fn from_index(index: ApiIndexV1) -> Result<Self, Error> {
        Ok(MappedIndex {
            api: index
                .items
                .into_iter()
                .map(Api::try_from)
                .collect::<Result<Vec<_>, Error>>()?,
        })
    }
}

pub fn execute(
    Args {
        discovery_json_path,
        output_file,
    }: Args,
) -> Result<(), Error> {
    let index: ApiIndexV1 = { serde_json::from_slice(&fs::read(&discovery_json_path)?) }
        .with_context(|_| {
            format_err!(
                "Could read spec file at '{}'",
                discovery_json_path.display()
            )
        })?;

    let index = MappedIndex::from_index(index)?;
    logged_write(
        output_file,
        serde_json::to_string_pretty(&index)?.as_bytes(),
        "mapped api index",
    )
}
