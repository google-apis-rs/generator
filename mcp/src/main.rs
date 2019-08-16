#[macro_use]
extern crate lazy_static;

const PROGRAM_NAME: &str = "mcp";

/// taken from share-secrets-safely/tools
mod options;

/// taken from share-secrets-safely/tools
mod parse;

use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, App, AppSettings,
    ArgMatches,
};

fn main() {
    let app: App = app_from_crate!()
        .setting(AppSettings::ColoredHelp)
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::DeriveDisplayOrder)
        .setting(AppSettings::SubcommandRequired)
        .name(PROGRAM_NAME)
        .after_help("Also join us on gitter: https://gitter.im/google-apis-rs/community")
        .subcommand(crate::options::substitute::new())
        .subcommand(crate::options::process::new())
        .subcommand(crate::options::completions::new());
    let app_clone = app.clone();
    let matches: ArgMatches = app.get_matches();

    let res = match matches.subcommand() {
        ("completions", Some(args)) => parse::completions::generate(app_clone, args),
        ("process", Some(args)) => parse::process::execute(args),
        ("substitute", Some(args)) => parse::substitute::execute(args),
        _ => panic!("Expected clap to prevent this"),
    };

    failure_tools::ok_or_exit(res);
}
