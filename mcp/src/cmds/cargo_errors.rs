use crate::options::cargo_errors::Args;
use cargo_log_parser::parse_errors;
use failure::{format_err, Error, ResultExt};
use std::process::{Command, Stdio};

use std::io::{self, Read};

pub fn feed_parser_and_tee_to<'a, P, T, E>(
    parser: P,
    mut rdr: impl io::Read + 'a,
    out: impl io::Write,
) -> Result<T, E>
where
    P: Fn(&'a [u8]) -> Result<T, nom::Err<E>>,
{
    let mut input = Vec::new();
    loop {
        let to_read = match parser(&input) {
            Ok(parsed) => {
                continue;
            }
            Err(nom::Err::Incomplete(needed)) => {
                match needed {
                    nom::Needed::Unknown => 1, // read one byte
                    nom::Needed::Size(len) => len,
                }
            }
            Err(nom::Err::Failure(e)) | Err(nom::Err::Error(e)) => return Err(e),
        };

        if let Err(e) = (&mut rdr).take(to_read as u64).read_to_end(&mut input) {
            if e.kind() == io::ErrorKind::BrokenPipe {
                return match parser(&input) {
                    Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => Err(e),
                    Err(nom::Err::Incomplete(_)) => {
                        panic!("Could not parse remaining input of length {}", input.len())
                    }
                    Ok(r) => Ok(r),
                };
            }
            unimplemented!("have to improved error handling!")
        }
    }
}

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
    let stdout = cargo.stdout.expect("stdout is set");
    feed_parser_and_tee_to(parse_errors, stdout, io::stdout())
        .map_err(|(_, e)| format_err!("{:?}", e))?;
    unimplemented!()
}
