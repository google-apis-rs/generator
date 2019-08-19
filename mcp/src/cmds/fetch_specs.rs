use super::util::{log_error_and_continue, logged_write};
use crate::options::fetch_specs::Args;
use discovery_parser::{
    generated::{ApiIndexV1, Item},
    DiscoveryRestDesc, RestDescOrErr,
};
use failure::{format_err, Error, ResultExt};
use log::info;
use rayon::prelude::*;
use std::{fs, path::Path, time::Instant};

fn write_artifacts<'a>(
    spec: DiscoveryRestDesc,
    output_dir: &Path,
) -> Result<DiscoveryRestDesc, Error> {
    let output_dir = output_dir.join(&spec.name).join(&spec.version);
    fs::create_dir_all(&output_dir).with_context(|_| {
        format_err!(
            "Could not create artifact output directory at '{}'",
            output_dir.display()
        )
    })?;

    let spec_path = output_dir.join("spec.json");
    // TODO: if no additional processing is done on the data, just pass it as String to avoid
    // ser-de. This is not relevant for performance, but can simplify code a bit.
    logged_write(
        &spec_path,
        serde_json::to_string_pretty(&spec)?.as_bytes(),
        "spec",
    )?;
    Ok(spec)
}

fn fetch_spec(api: &Item) -> Result<DiscoveryRestDesc, Error> {
    reqwest::get(&api.discovery_rest_url)
        .with_context(|_| format_err!("Could not fetch spec from '{}'", api.discovery_rest_url))
        .map_err(Error::from)
        .and_then(|mut r: reqwest::Response| {
            let res: RestDescOrErr = r.json().with_context(|_| {
                format_err!("Could not deserialize spec at '{}'", api.discovery_rest_url)
            })?;
            match res {
                RestDescOrErr::RestDesc(v) => Ok(v),
                RestDescOrErr::Err(err) => Err(format_err!("{:?}", err.error)),
            }
        })
        .with_context(|_| format_err!("Error fetching spec from '{}'", api.discovery_rest_url))
        .map_err(Into::into)
}

pub fn execute(
    Args {
        discovery_json_path,
        output_directory,
    }: Args,
) -> Result<(), Error> {
    let apis: ApiIndexV1 =
        serde_json::from_str(&fs::read_to_string(&discovery_json_path).with_context(|_| {
            format_err!(
                "Could not read api index at '{}'",
                discovery_json_path.display()
            )
        })?)?;

    let time = Instant::now();
    apis.items
        .par_iter()
        .map(fetch_spec)
        .filter_map(log_error_and_continue)
        .map(|v| write_artifacts(v, &output_directory))
        .filter_map(log_error_and_continue)
        .for_each(|api| info!("Successfully processed {}:{}", api.name, api.version));
    info!(
        "Fetched {} specs in {}s",
        apis.items.len(),
        time.elapsed().as_secs()
    );
    Ok(())
}
