use crate::options::cargo_errors::Args;
use cargo_log_parser::parse_errors;
use failure::{bail, format_err, Error, ResultExt};
use log::{error, info};
use shared::MappedIndex;
use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
    process::{Command, Stdio},
};

pub fn execute(
    Args {
        index_path,
        cargo_manifest_path,
        output_directory,
        mut cargo_arguments,
    }: Args,
) -> Result<(), Error> {
    let mut excludes = Vec::new();
    let mut last_excludes_len = 0;
    let index: MappedIndex = serde_json::from_slice(&fs::read(index_path)?)?;
    let filter_parse_result = |parsed: Vec<cargo_log_parser::CrateWithError>| {
        parsed
            .into_iter()
            .map(|c| c.name)
            .filter(|n| index.api.iter().any(|api| &api.crate_name == n))
    };

    loop {
        cargo_arguments.push("--manifest-path".into());
        cargo_arguments.push(cargo_manifest_path.clone().into());
        cargo_arguments.extend(
            excludes
                .iter()
                .map(|crate_name| format!("--exclude={}", crate_name).into()),
        );
        let mut cargo = Command::new("cargo")
            .args(&cargo_arguments)
            .stderr(Stdio::piped())
            .stdout(Stdio::inherit())
            .stdin(Stdio::null())
            .spawn()
            .with_context(|_| "failed to launch cargo")?;

        let mut input = Vec::new();
        let mut print_from = 0_usize;
        loop {
            let written_bytes = io::stderr().write(&input[print_from..])?;
            print_from = written_bytes;

            let to_read = match parse_errors(&input).map(|(i, r)| (i.len(), r)) {
                Ok((input_left_len, parsed)) => {
                    dbg!(&parsed);
                    let input_len = input.len();
                    input = input.into_iter().skip(input_len - input_left_len).collect();
                    print_from = 0;
                    excludes.extend((filter_parse_result)(parsed));
                    128
                }
                Err(nom::Err::Incomplete(needed)) => {
                    match needed {
                        nom::Needed::Unknown => 1, // read one byte
                        nom::Needed::Size(len) => len,
                    }
                }
                Err(nom::Err::Failure(_e)) | Err(nom::Err::Error(_e)) => {
                    bail!("TODO: proper error conversion if parsing really fails")
                }
            };

            if let Some(_) = cargo.try_wait()? {
                break;
            }

            if let Err(e) = cargo
                .stderr
                .as_mut()
                .expect("cargo_output is set")
                .take(to_read as u64)
                .read_to_end(&mut input)
            {
                error!("Failed to read cargo output: {}", e);
                break;
            }
        }

        cargo
            .stderr
            .as_mut()
            .expect("cargo_output is set")
            .read_to_end(&mut input)?;

        io::stderr().write(&input[print_from..])?;

        match parse_errors(&input) {
            Ok((_, parsed)) => {
                dbg!(&parsed);
                excludes.extend(filter_parse_result(parsed));
            }
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                error!("Ignoring parse error after cargo ended: {:?}", e.1);
            }
            Err(nom::Err::Incomplete(_)) => panic!(
                "Could not parse remaining input: {:?}",
                std::str::from_utf8(&input)
            ),
        };

        collect_errors(
            &excludes[last_excludes_len..],
            &index,
            output_directory.as_path(),
        )?;

        let workspace_cargo_status = cargo.try_wait()?.expect("cargo ended");

        if workspace_cargo_status.success() {
            info!("Cargo finished successfully.");
            if !excludes.is_empty() {
                info!(
                    "Recorded errors for the following workspace members: {:?}",
                    excludes
                );
            }
            return Ok(());
        } else {
            if last_excludes_len == excludes.len() {
                bail!(
                    "cargo seems to fail permanently and makes no progress: {:?}",
                    workspace_cargo_status
                );
            }
            last_excludes_len = excludes.len();
        }
    }
}

fn collect_errors(
    crate_names: &[String],
    index: &MappedIndex,
    output_directory: &Path,
) -> Result<(), Error> {
    for crate_name in crate_names {
        let api = index
            .api
            .iter()
            .find(|api| &api.crate_name == crate_name)
            .ok_or_else(|| {
                format_err!(
                    "Could not find crate named '{}' to collect errors",
                    crate_name
                )
            })?;
    }
    unimplemented!("todo :collect errors of crate");
}
