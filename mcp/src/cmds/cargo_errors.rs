use crate::options::cargo_errors::Args;
use cargo_log_parser::parse_errors;
use failure::{bail, Error, ResultExt};
use std::process::{Command, Stdio};

use std::io::{self, Read};

pub fn execute(
    Args {
        index_path,
        cargo_manifest_path,
        output_directory,
        cargo_arguments,
    }: Args,
) -> Result<(), Error> {
    let cargo = Command::new("cargo")
        .args(cargo_arguments)
        .stderr(Stdio::piped())
        .stdout(Stdio::inherit())
        .stdin(Stdio::null())
        .spawn()
        .with_context(|_| "failed to launch cargo")?;
    let mut stdout = cargo.stdout.expect("stdout is set");

    let mut input = Vec::new();
    loop {
        let to_read = match parse_errors(&input) {
            Ok(parsed) => {
                dbg!(parsed);
                continue;
            }
            Err(nom::Err::Incomplete(needed)) => {
                match needed {
                    nom::Needed::Unknown => 1, // read one byte
                    nom::Needed::Size(len) => len,
                }
            }
            Err(nom::Err::Failure(e)) | Err(nom::Err::Error(e)) => {
                bail!("TODO: proper error conversion if parsing really fails")
            }
        };

        if let Err(e) = (&mut stdout).take(to_read as u64).read_to_end(&mut input) {
            if e.kind() == io::ErrorKind::BrokenPipe {
                return match parse_errors(&input) {
                    Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                        bail!("TODO: proper error conversion")
                    }
                    Err(nom::Err::Incomplete(_)) => {
                        panic!("Could not parse remaining input of length {}", input.len())
                    }
                    Ok(parsed) => {
                        dbg!(parsed);
                        return Ok(());
                    }
                };
            }
            unimplemented!("have to improved error handling!")
        }
    }
    unimplemented!()
}
