use std::ffi::OsString;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(setting = structopt::clap::AppSettings::ColoredHelp)]
pub struct Args {
    /// The name of the shell, or the path to the shell as exposed by the $SHELL variable
    #[structopt(parse(from_os_str))]
    pub shell: OsString,
}
