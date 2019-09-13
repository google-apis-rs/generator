use crate::options::cargo_errors::Args;
use cargo_log_parser::parse_errors;
use failure::{bail, format_err, Error, ResultExt};
use log::{error, info, warn};
use shared::{Api, MappedIndex};
use std::{
    ffi::OsString,
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
        cargo_arguments,
    }: Args,
) -> Result<(), Error> {
    let mut last_excludes_len = 0;
    let index: MappedIndex = serde_json::from_slice(&fs::read(index_path)?)?;
    let mut excludes = Vec::<&Api>::new();
    let filter_parse_result = |parsed: Vec<cargo_log_parser::CrateWithError>| {
        parsed
            .into_iter()
            .map(|c| c.name)
            .filter_map(|n| index.api.iter().find(|api| api.lib_crate_name == n))
    };

    loop {
        let mut args = cargo_arguments.clone();
        args.push("--all".into());
        args.push("--manifest-path".into());
        args.push(cargo_manifest_path.clone().into());
        args.extend(
            excludes
                .iter()
                .map(|api| format!("--exclude={}", api.lib_crate_name).into()),
        );
        command_info("<workspace>", &args);
        let mut cargo = Command::new("cargo")
            .args(&args)
            .stderr(Stdio::piped())
            .stdout(Stdio::inherit())
            .stdin(Stdio::null())
            .spawn()
            .with_context(|_| "failed to launch cargo")?;

        let mut input = Vec::new();
        let mut print_from = 0_usize;
        loop {
            let written_bytes = io::stderr().write(&input[print_from..])?;
            print_from += written_bytes;

            let to_read = match parse_errors(&input).map(|(i, r)| (i.len(), r)) {
                Ok((input_left_len, parsed)) => {
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
            &cargo_arguments,
            output_directory.as_path(),
        )?;

        let workspace_cargo_status = cargo.try_wait()?.expect("cargo ended");

        if workspace_cargo_status.success() {
            info!("Cargo finished successfully.");
            if !excludes.is_empty() {
                info!(
                    "Recorded errors for the following workspace members: {:?}",
                    excludes
                        .iter()
                        .map(|a| &a.lib_crate_name)
                        .collect::<Vec<_>>()
                );
            }
            return Ok(());
        } else {
            if last_excludes_len == excludes.len() {
                bail!(
                    "cargo seems to fail permanently and makes no progress: {:?}. Probably we cannot parse crate names from the error output.",
                    workspace_cargo_status
                );
            }
            last_excludes_len = excludes.len();
        }
    }
}

fn command_info(prefix: &str, args: &[OsString]) {
    info!(
        "({}) Running 'cargo {}'",
        prefix,
        args.iter()
            .map(|o| o.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
    );
}

fn collect_errors(
    apis: &[&Api],
    cargo_arguments: &[OsString],
    output_directory: &Path,
) -> Result<(), Error> {
    for api in apis {
        let mut args = cargo_arguments.to_owned();
        let manifest_path = output_directory.join(&api.lib_cargo_file);
        args.push("--manifest-path".into());
        args.push(manifest_path.into());
        command_info(&api.lib_crate_name, &args);

        let output = Command::new("cargo")
            .args(&args)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .stdin(Stdio::null())
            .output()
            .with_context(|_| "failed to launch cargo and collect output")?;
        let output_path = output_directory.join(&api.cargo_error_file);
        if output.status.success() {
            warn!("Command succeeded unexpectedly - no file error log written.");
            continue;
        }
        let mut fh = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&output_path)
            .with_context(|_| {
                format_err!("Could not output path at '{}'", output_path.display())
            })?;
        fh.write_all(&output.stderr)?;
        fh.write_all(&output.stdout)?;
        fh.flush()?;
        info!("Wrote cargo error log to '{}'", output_path.display());
    }
    Ok(())
}
