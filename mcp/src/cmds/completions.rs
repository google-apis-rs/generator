use crate::options::completions::Args;
use clap::{App, Shell};
use failure::{err_msg, format_err, Error, ResultExt};
use std::{io::stdout, path::Path, str::FromStr};

pub fn execute(mut app: App, Args { shell }: Args) -> Result<(), Error> {
    let shell = Path::new(&shell)
        .file_name()
        .and_then(|f| f.to_str())
        .or_else(|| shell.to_str())
        .ok_or_else(|| {
            format_err!(
                "'{}' as shell string contains invalid characters",
                shell.to_string_lossy()
            )
        })
        .and_then(|s| {
            Shell::from_str(s)
                .map_err(err_msg)
                .with_context(|_| format!("The shell '{}' is unsupported", s))
                .map_err(Into::into)
        })?;
    app.gen_completions_to(crate::PROGRAM_NAME, shell, &mut stdout());
    Ok(())
}
