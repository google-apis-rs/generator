use crate::options::fetch_specs::Args;
use discovery_parser::{
    generated::{DiscoveryApisV1, Item},
    DiscoveryRestDesc, RestDescOrErr,
};
use failure::{bail, format_err, Error, ResultExt};
use failure_tools::print_causes;
use log::{error, info};
use rayon::prelude::*;
use std::{convert::TryFrom, convert::TryInto, fs, path::Path, time::Instant};

#[derive(Debug, PartialEq, Eq)]
struct Id {
    name: String,
    version: String,
}

impl TryFrom<&str> for Id {
    type Error = Error;

    fn try_from(s: &str) -> Result<Id, Error> {
        let mut tokens = s.rsplit(':');
        match (tokens.next(), tokens.next()) {
            (Some(v), Some(n)) => Ok(Id {
                name: n.into(),
                version: v.into(),
            }),
            _ => bail!("Could not parse '{}' as id like 'name:version'", s),
        }
    }
}

#[derive(Debug)]
struct Api {
    id: Id,
    url: String,
}

impl<'a> TryFrom<&'a Item> for Api {
    type Error = Error;

    fn try_from(value: &Item) -> Result<Api, Error> {
        Ok(Api {
            id: value.id.as_str().try_into()?,
            url: value.discovery_rest_url.clone(),
        })
    }
}

fn log_error_and_continue<T, E: Into<Error>>(r: Result<T, E>) -> Option<T> {
    match r {
        Ok(v) => Some(v),
        Err(e) => {
            let e = e.into();
            let mut buf = Vec::new();
            let e_display = e.to_string();
            print_causes(e, &mut buf);
            error!("{}", String::from_utf8(buf).unwrap_or(e_display));
            None
        }
    }
}

fn logged_write<P: AsRef<Path>, C: AsRef<[u8]>>(
    path: P,
    contents: C,
    kind: &str,
) -> Result<(), Error> {
    fs::write(path.as_ref(), contents).with_context(|_| {
        format_err!(
            "Could not write {kind} file at '{}'",
            path.as_ref().display(),
            kind = kind,
        )
    })?;
    info!(
        "Wrote file {kind} at '{}'",
        path.as_ref().display(),
        kind = kind
    );
    Ok(())
}

fn write_artifacts<'a>(
    (api, spec): (Api, DiscoveryRestDesc),
    output_dir: &Path,
) -> Result<Api, Error> {
    let output_dir = output_dir.join(&api.id.name).join(&api.id.version);
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
    Ok(api)
}

pub fn execute(
    Args {
        discovery_json_path,
        output_directory,
    }: Args,
) -> Result<(), Error> {
    let apis: DiscoveryApisV1 =
        serde_json::from_str(&fs::read_to_string(&discovery_json_path).with_context(|_| {
            format_err!(
                "Could not read api index at '{}'",
                discovery_json_path.display()
            )
        })?)?;

    let time = Instant::now();
    apis.items
        .par_iter()
        .map(Api::try_from)
        .filter_map(log_error_and_continue)
        .map(|api| {
            reqwest::get(&api.url)
                .with_context(|_| format_err!("Could not fetch spec from '{}'", api.url))
                .and_then(|mut r: reqwest::Response| {
                    let res: RestDescOrErr = r.json().with_context(|_| {
                        format_err!("Could not deserialize spec at '{}'", api.url)
                    })?;
                    match res {
                        RestDescOrErr::RestDesc(v) => Ok(v),
                        RestDescOrErr::Err(err) => Err(format_err!("{:?}", err.error))
                            .with_context(|_| format_err!("Server responded with an error")),
                    }
                })
                .with_context(|_| format_err!("Error fetching spec from '{}'", api.url))
                .map(|spec: DiscoveryRestDesc| (api, spec))
        })
        .filter_map(log_error_and_continue)
        .map(|v| write_artifacts(v, &output_directory))
        .filter_map(log_error_and_continue)
        .for_each(|api| info!("Successfully processed ${:?}", api));
    info!(
        "Processed {} specs in {}s",
        apis.items.len(),
        time.elapsed().as_secs()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod id {
        use super::*;

        #[test]
        fn valid_ids_work() {
            assert_eq!(
                Id {
                    name: "name".into(),
                    version: "version".into()
                },
                Id::try_from("name:version").unwrap()
            )
        }

        #[test]
        fn invalid_ids_yield_an_error() {
            assert!(Id::try_from("nameversion").is_err())
        }
    }
}
