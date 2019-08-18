use clap::{App, Arg, ArgSettings};
use std::ffi::OsString;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
#[structopt(raw(setting = "structopt::clap::AppSettings::AllowLeadingHyphen"))]
#[structopt(raw(alias = "\"sub\""))]
/// Substitutes templates using structured data.
///
/// The idea is to build a tree of data that is used to substitute in various templates, using multiple inputs and outputs.
/// That way, secrets (like credentials) can be extracted from the vault just once and used wherever needed without them touching disk.
/// Liquid is used as template engine, and it's possible to refer to and inherit from other templates by their file-stem.
/// Read more on their website at https://shopify.github.io/liquid .
pub struct Args {
    #[structopt(raw(set = "ArgSettings::RequireEquals"))]
    #[structopt(short = "e", long = "engine", name = "name", default_value = "liquid")]
    #[structopt(raw(possible_values = r#"&["handlebars", "liquid"]"#))]
    /// The choice of engine used for the substitution.
    ///
    /// 'liquid', is coming with batteries included and very good at handling
    /// one template at a time.
    /// 'handlebars' supports referencing other templates using partials, which
    /// is useful for sharing of common functionality.
    engine: String,

    #[structopt(parse(from_os_str))]
    #[structopt(raw(set = "ArgSettings::RequireEquals"))]
    #[structopt(
        short = "s",
        long = "separator",
        name = "separator",
        default_value = "\n"
    )]
    /// The string to use to separate multiple documents that are written to the same stream.
    ///
    /// This can be useful to output a multi-document YAML file from multiple input templates
    /// to stdout if the separator is '---'.
    /// The separator is also used when writing multiple templates into the same file, like in 'a:out b:out'.
    separator: OsString,

    #[structopt(raw(set = "ArgSettings::RequireEquals"))]
    #[structopt(raw(use_delimiter = "true"))]
    #[structopt(
        long = "replace",
        value_delimiter = ":",
        value_name = "find-this:replace-with-that"
    )]
    /// A simple find & replace for values for the string data to be placed into the template. \
    /// The word to find is the first specified argument, the second one is the word to replace it with, \
    /// e.g. -r=foo:bar.
    replacements: Vec<String>,
}

pub fn _new<'a, 'b>() -> App<'a, 'b> {
    App::new("substitute")
        .arg(
            Arg::with_name("validate")
                .required(false)
                .long("validate")
                .short("v")
                .help("If set, the instantiated template will be parsed as YAML or JSON. \
               If both of them are invalid, the command will fail.")
        )
        .arg(
            Arg::with_name("data")
                .set(ArgSettings::RequireEquals)
                .required(false)
                .multiple(false)
                .takes_value(true)
                .long("data")
                .short("d")
                .value_name("data")
                .help("Structured data in YAML or JSON format to use when instantiating/substituting the template. \
               If set, everything from standard input is interpreted as template."),
        )
        .arg(
            Arg::with_name("spec")
                .required(false)
                .multiple(true)
                .takes_value(true)
                .value_name("template-spec")
                .long_help("Identifies the how to map template files to output. \
             The syntax is '<src>:<dst>'. \
             <src> and <dst> are a relative or absolute paths to the source templates or \
             destination files respectively. \
             If <src> is unspecified, the template will be read from stdin, e.g. ':output'. Only one spec can read from stdin. \
             If <dst> is unspecified, the substituted template will be output to stdout, e.g 'input.hbs:' \
             or 'input.hbs'. Multiple templates are separated by the '--separator' accordingly. This is particularly useful for YAML files,\
             where the separator should be `$'---\\n'`",
                ),
        )
}
