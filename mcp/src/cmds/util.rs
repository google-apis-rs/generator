use failure::{format_err, Error, ResultExt};
use failure_tools::print_causes;
use log::{error, info};
use std::{fs, path::Path};

pub fn log_error_and_continue<T, E: Into<Error>>(r: Result<T, E>) -> Option<T> {
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

pub fn logged_write<P: AsRef<Path>, C: AsRef<[u8]>>(
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
