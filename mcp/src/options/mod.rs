pub fn _output_formats() -> &'static [&'static str] {
    &["json", "yaml"]
}

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
#[structopt(raw(setting = "structopt::clap::AppSettings::VersionlessSubcommands"))]
#[structopt(raw(setting = "structopt::clap::AppSettings::DeriveDisplayOrder"))]
pub struct Args {
    /// The desired log level.
    #[structopt(short = "l", long = "log-level", default_value = "INFO")]
    #[structopt(raw(possible_values = r#"&["INFO", "ERROR", "DEBUG"]"#))]
    pub log_level: log::Level,
    #[structopt(subcommand)]
    pub(crate) cmd: SubCommand,
}

#[derive(StructOpt, Debug)]
pub enum SubCommand {
    #[structopt(name = "fetch-api-specs")]
    /// Fetch all API specs, in parallel
    FetchApiSpecs(fetch_specs::Args),
    #[structopt(name = "completions")]
    /// generate completions for supported shells
    Completions(completions::Args),
    #[structopt(name = "generate")]
    /// generate APIs and CLIs for a Google API specification
    Generate(generate::Args),
    #[structopt(name = "substitute")]
    #[structopt(raw(alias = "\"sub\""))]
    /// Substitutes templates using structured data.
    Substitute(substitute::Args),
}

pub mod completions;
pub mod fetch_specs;
pub mod generate;
pub mod substitute;
