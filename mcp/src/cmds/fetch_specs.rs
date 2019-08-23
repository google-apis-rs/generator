use super::util::{log_error_and_continue, logged_write};
use crate::options::fetch_specs::Args;
use discovery_parser::{generated::ApiIndexV1, DiscoveryRestDesc, RestDescOrErr};
use failure::{format_err, Error, ResultExt};
use log::info;
use rayon::prelude::*;
use shared::{Api, MappedIndex};
use std::{convert::TryInto, fs, path::Path, time::Instant};

fn write_artifacts(
    api: &Api,
    spec: DiscoveryRestDesc,
    output_dir: &Path,
) -> Result<DiscoveryRestDesc, Error> {
    let spec_path = output_dir.join(&api.spec_file);
    fs::create_dir_all(
        &spec_path
            .parent()
            .ok_or_else(|| format_err!("invalid spec path - needs parent"))?,
    )
    .with_context(|_| {
        format_err!(
            "Could not create artifact output directory at '{}'",
            output_dir.display()
        )
    })?;

    // TODO: if no additional processing is done on the data, just pass it as String to avoid
    // ser-de. This is not relevant for performance, but can simplify code a bit.
    logged_write(
        &spec_path,
        serde_json::to_string_pretty(&spec)?.as_bytes(),
        "spec",
    )?;
    Ok(spec)
}

fn fetch_spec(api: &Api) -> Result<DiscoveryRestDesc, Error> {
    reqwest::get(&api.rest_url)
        .with_context(|_| format_err!("Could not fetch spec from '{}'", api.rest_url))
        .map_err(Error::from)
        .and_then(|mut r: reqwest::Response| {
            let res: RestDescOrErr = r.json().with_context(|_| {
                format_err!("Could not deserialize spec at '{}'", api.rest_url)
            })?;
            match res {
                RestDescOrErr::RestDesc(v) => Ok(v),
                RestDescOrErr::Err(err) => Err(format_err!("{:?}", err.error)),
            }
        })
        .with_context(|_| format_err!("Error fetching spec from '{}'", api.rest_url))
        .map_err(Into::into)
}

pub fn execute(
    Args {
        index_path,
        spec_directory,
    }: Args,
) -> Result<(), Error> {
    let input = fs::read_to_string(&index_path)?;
    let index: MappedIndex = serde_json::from_str::<ApiIndexV1>(&input)
        .map_err(Error::from)
        .and_then(TryInto::try_into)
        .or_else(|_| serde_json::from_str(&input))
        .map_err(Error::from)
        .with_context(|_| {
            format_err!(
                "Could not read google api index at '{}'",
                index_path.display()
            )
        })?;
    let time = Instant::now();
    index
        .api
        .par_iter()
        .map(|api| fetch_spec(api).map(|r| (api, r)))
        .filter_map(log_error_and_continue)
        .map(|(api, v)| write_artifacts(api, v, &spec_directory))
        .filter_map(log_error_and_continue)
        .for_each(|api| info!("Successfully processed {}:{}", api.name, api.version));
    info!(
        "Fetched {} specs in {}s",
        index.api.len(),
        time.elapsed().as_secs()
    );
    Ok(())
}
