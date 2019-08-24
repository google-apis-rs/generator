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
    let cargo = Command::new("cargo")
        .args(cargo_arguments)
        .stderr(Stdio::piped())
        .stdout(Stdio::inherit())
        .stdin(Stdio::null())
        .spawn()
        .with_context(|_| "failed to launch cargo")?;
    let mut cargo_output = cargo.stderr.expect("cargo_output is set");

    let mut input = Vec::new();
    loop {
        io::stderr().write(&input).ok();
        let to_read = match parse_errors(&input).map(|(i, r)| (i.len(), r)) {
            Ok((input_left_len, parsed)) => {
                dbg!(parsed);
                let input_len = input.len();
                input = input.into_iter().skip(input_len - input_left_len).collect();
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

        if let Err(e) = (&mut cargo_output)
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
