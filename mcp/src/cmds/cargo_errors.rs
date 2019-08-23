use crate::options::cargo_errors::Args;
use cargo_log_parser::parse_errors;
use failure::{Error, ResultExt};
use std::process::{Command, Stdio};

use std::io::{self, Read};

pub fn feed_parser_and_tee_to<P, T, E, Any>(
    parser: P,
    mut rdr: impl io::Read,
    out: impl io::Write,
) -> Result<T, E>
where
    E: From<io::Error>,
    P: Fn(&[u8]) -> Result<T, nom::Err<E>>,
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

        (&mut rdr).take(to_read as u64).read_to_end(&mut input)?;
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
    feed_parser_and_tee_to(
        |i: &[u8]| {
            parse_errors(i).map(|c| {
                dbg!(c);
                ()
            })
        },
        stdout,
        io::stdout(),
    )?;
    unimplemented!()
}
