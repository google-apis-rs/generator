use super::util::{log_error_and_continue, logged_write};
use crate::options::fetch_specs::Args;
use ci_info;
use discovery_parser::{generated::ApiIndexV1, DiscoveryRestDesc, RestDescOrErr};
use failure::{err_msg, format_err, Error, ResultExt};
use google_rest_api_generator::{generate, Metadata};
use log::info;
use rayon::prelude::*;
use shared::{Api, MappedIndex, SkipIfErrorIsPresent};
use std::convert::TryFrom;
use std::{convert::TryInto, fs, io, path::Path, time::Instant};

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

fn generate_code(
    desc: DiscoveryRestDesc,
    info: &ci_info::types::CiInfo,
    spec_directory: &Path,
    output_directory: &Path,
) -> Result<DiscoveryRestDesc, Error> {
    let api = Api::try_from(&desc)?.validated(
        info,
        spec_directory,
        output_directory,
        SkipIfErrorIsPresent::Generator,
    )?;
    let should_generate = (|| -> Result<_, Error> {
        let cargo_path = output_directory.join(&api.lib_cargo_file);
        if !cargo_path.exists() {
            info!(
                "Need to generate '{}' as it was never generated before.",
                api.crate_name
            );
            return Ok(true);
        }
        let metadata_path = output_directory.join(&api.metadata_file);
        let previous_metadata = fs::read(&metadata_path)
            .map_err(Error::from)
            .and_then(|data| serde_json::from_slice::<Metadata>(&data).map_err(Error::from))
            .unwrap_or_else(|_| Metadata {
                git_hash: "no data yet".into(),
                ymd_date: "no data yet".into(),
            });
        let current_metadata = Metadata::default();
        if previous_metadata != current_metadata {
            info!("Generator changed for '{}'. Last generated content stamped with {:?}, latest version is {:?}", api.crate_name, previous_metadata, current_metadata);
            return Ok(true);
        }
        let spec_path = spec_directory.join(&api.spec_file);
        let buf = match fs::read(&spec_path) {
            Ok(v) => Ok(v),
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => return Ok(true),
            Err(e) => Err(e).with_context(|_| {
                format_err!("Could not read spec file at '{}'", spec_path.display())
            }),
        }?;
        let local_api: DiscoveryRestDesc = serde_json::from_slice(&buf)?;
        Ok(local_api != desc)
    })()?;
    if !should_generate {
        info!("Skipping generation of '{}' as it is up to date", api.id);
        return Ok(desc);
    }
    generate(&api.crate_name, &desc, output_directory.join(&api.gen_dir)).map_err(|e| {
        let error = e.to_string();
        let error_path = output_directory.join(api.gen_error_file);
        fs::write(&error_path, &error).ok();
        info!(
            "Api '{}' failed to generate, marked it at '{}'",
            api.id,
            error_path.display()
        );
        err_msg(error)
    })?;
    Ok(desc)
}

pub fn execute(
    Args {
        index_path,
        spec_directory,
        output_directory,
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
    let info = ci_info::get();
    index
        .api
        .par_iter()
        .map(|api| fetch_spec(api).map(|r| (api, r)))
        .filter_map(log_error_and_continue)
        .map(|(api, v)| write_artifacts(api, v, &spec_directory))
        .filter_map(log_error_and_continue)
        .map(|api| generate_code(api, &info, &spec_directory, &output_directory))
        .filter_map(log_error_and_continue)
        .for_each(|api| info!("Successfully processed {}:{}", api.name, api.version));
    info!(
        "Fetched and generated {} specs in {}s",
        index.api.len(),
        time.elapsed().as_secs()
    );
    Ok(())
}
