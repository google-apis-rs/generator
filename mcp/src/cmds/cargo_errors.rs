use crate::options::cargo_errors::Args;
use cargo_log_parser::parse_errors;
use failure::{bail, Error, ResultExt};
use std::{
    io::{self, Read, Write},
    process::{Command, Stdio},
};

pub fn execute(
    Args {
        index_path: _,
        cargo_manifest_path,
        output_directory: _,
        mut cargo_arguments,
    }: Args,
) -> Result<(), Error> {
    cargo_arguments.push("--manifest-path".into());
    cargo_arguments.push(cargo_manifest_path.into());
    let mut cargo = Command::new("cargo")
        .args(cargo_arguments)
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
                dbg!(parsed);
                let input_len = input.len();
                input = input.into_iter().skip(input_len - input_left_len).collect();
                print_from = 0;
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

        if let Some(status) = cargo.try_wait()? {
            if input.len() > 0 {
                unimplemented!("parse remaining bytes - avoid all that code duplication");
            }
            if status.success() {
                return Ok(());
            } else {
                bail!("cargo exited with error: {:?}", status)
            }
        }

        if let Err(e) = cargo
            .stderr
            .as_mut()
            .expect("cargo_output is set")
            .take(to_read as u64)
            .read_to_end(&mut input)
        {
            if e.kind() == io::ErrorKind::BrokenPipe {
                match parse_errors(&input) {
                    Ok(parsed) => {
                        dbg!(parsed);
                        return Ok(());
                    }
                    Err(nom::Err::Error(_e)) | Err(nom::Err::Failure(_e)) => {
                        bail!("TODO: proper error conversion")
                    }
                    Err(nom::Err::Incomplete(_)) => {
                        panic!("Could not parse remaining input of length {}", input.len())
                    }
                };
            }
            unimplemented!("have to improve error handling!")
        }
    }
}
