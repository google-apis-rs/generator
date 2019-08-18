pub fn _output_formats() -> &'static [&'static str] {
    &["json", "yaml"]
}

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
#[structopt(raw(setting = "structopt::clap::AppSettings::VersionlessSubcommands"))]
#[structopt(raw(setting = "structopt::clap::AppSettings::DeriveDisplayOrder"))]
pub struct Args {
    #[structopt(subcommand)]
    pub(crate) cmd: SubCommand,
}

#[derive(StructOpt, Debug)]
pub enum SubCommand {
    #[structopt(name = "fetch-apis")]
    /// Fetch all API specs, in parallel
    FetchSpecs(fetch_specs::Args),
    #[structopt(name = "completions")]
    /// generate completions for supported shells
    Completions(completions::Args),
    #[structopt(name = "substitute")]
    #[structopt(raw(alias = "\"sub\""))]
    /// Substitutes templates using structured data.
    Substitute(substitute::Args),
}

pub mod completions;
pub mod fetch_specs;
pub mod substitute;
