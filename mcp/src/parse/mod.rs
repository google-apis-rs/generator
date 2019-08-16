use clap::ArgMatches;
use failure::{format_err, Error};
use std::ffi::OsStr;

pub fn required_os_arg<'a, T>(args: &'a ArgMatches, name: &'static str) -> Result<T, Error>
where
    T: From<&'a OsStr>,
{
    match args.value_of_os(name).map(Into::into) {
        Some(t) => Ok(t),
        None => Err(format_err!(
            "BUG: expected clap argument '{}' to be set",
            name
        )),
    }
}

pub fn optional_args_with_value<F, T>(
    args: &ArgMatches,
    name: &'static str,
    into: F,
) -> Vec<(T, usize)>
where
    F: Fn(&str) -> T,
{
    if args.occurrences_of(name) > 0 {
        match (args.values_of(name), args.indices_of(name)) {
            (Some(v), Some(i)) => v.map(|v| into(v)).zip(i).collect(),
            (None, None) => Vec::new(),
            _ => unreachable!("expecting clap to work"),
        }
    } else {
        Vec::new()
    }
}

pub mod completions;
pub mod process;
pub mod substitute;
