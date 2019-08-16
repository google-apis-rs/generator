#[macro_use]
extern crate lazy_static;
/// taken from share-secrets-safely/tools
mod options {
    pub fn output_formats() -> &'static [&'static str] {
        &["json", "yaml"]
    }

    pub mod process {
        use super::output_formats;
        use clap::{App, Arg, ArgSettings};
        use glob;

        pub fn new<'a, 'b>() -> App<'a, 'b> {
            App::new("process")
                .alias("show")
                .alias("merge")
                .about(
                    "Merge JSON or YAML files from standard input from specified files. \
             Multi-document YAML files are supported. \
             Merging a single file is explicitly valid and can be used to check for syntax errors.",
                )
                .arg(
                    Arg::with_name("select")
                        .set(ArgSettings::RequireEquals)
                        .alias("from")
                        .long("select")
                        .short("s")
                        .takes_value(true)
                        .value_name("pointer")
                        .required(false)
                        .multiple(true)
                        .help("Use a JSON pointer to specify which sub-value to use. \
                       This affects only the next following --environment or <path>. \
                       Valid specifications are for example '0/a/b/4' or 'a.b.0', and they must point to a valid value. \
                       If it is specified last, without a following merged value, a sub-value is selected from the aggregated value."
                        )
                )
                .arg(
                    Arg::with_name("no-stdin")
                        .long("no-stdin")
                        .required(false)
                        .help("If set, we will not try to read structured data from standard input. This may be required \
                       in some situations where we are blockingly reading from a standard input which is attached \
                       to a pseudo-terminal.")
                )
                .arg(
                    Arg::with_name("at")
                        .set(ArgSettings::RequireEquals)
                        .alias("to")
                        .long("at")
                        .short("a")
                        .takes_value(true)
                        .value_name("pointer")
                        .required(false)
                        .multiple(true)
                        .help("Use a JSON pointer to specify an existing mapping at which the next merged value should be placed. \
                       This affects only the next following --environment or <path>. \
                       Valid specifications are for example '0/a/b/4' or 'a.b.0'. \
                       If it is specified last, without a following merged value, the entire aggregated value so far is moved."
                        )
                )
                .arg(
                    Arg::with_name("environment")
                        .set(ArgSettings::RequireEquals)
                        .long("environment")
                        .short("e")
                        .takes_value(true)
                        .default_value("*")
                        .value_name("filter")
                        .required(false)
                        .multiple(true)
                        .validator(|v| glob::Pattern::new(&v).map(|_| ()).map_err(|err| format!("{}", err)))
                        .help("Import all environment variables matching the given filter. If no filter is set, all variables are imported. \
                       Otherwise it is applied as a glob, e.g. 'FOO*' includes 'FOO_BAR', but not 'BAZ_BAR'.\
                       Other valid meta characters are '?' to find any character, e.g. 'FO?' matches 'FOO'.")
                )
                .arg(
                    Arg::with_name("no-overwrite")
                        .alias("no-override")
                        .long("no-overwrite")
                        .takes_value(false)
                        .required(false)
                        .multiple(true)
                        .help("If set, values in the merged document may not overwrite values already present. This is enabled by default,\
                       and can be explicitly turned off with --overwrite."),
                )
                .arg(
                    Arg::with_name("overwrite")
                        .alias("override")
                        .long("overwrite")
                        .takes_value(false)
                        .required(false)
                        .multiple(true)
                        .help("If set, values in the merged document can overwrite values already present. This is disabled by default,\
                       and can be explicitly turned off with --no-overwrite."),
                )
                .arg(
                    Arg::with_name("output")
                        .set(ArgSettings::RequireEquals)
                        .short("o")
                        .long("output")
                        .takes_value(true)
                        .required(false)
                        .value_name("mode")
                        .default_value("json")
                        .possible_values(output_formats())
                        .case_insensitive(true)
                        .help("Specifies how the merged result should be serialized."),
                )
                .arg(
                    Arg::with_name("path")
                        .value_name("path-or-value")
                        .takes_value(true)
                        .required(false)
                        .multiple(true)
                        .help(
                            "The path to the file to include, or '-' to read from standard input. It must be in a format that can be output using the --output flag. \
                 Alternatively it can be a value assignment like 'a=42' or a.b.c=value.",
                        ),
                )
        }
    }

    pub mod substitute {
        use clap::AppSettings;
        use clap::ArgSettings;
        use clap::{App, Arg};

        pub fn new<'a, 'b>() -> App<'a, 'b> {
            App::new("substitute")
                .setting(AppSettings::AllowLeadingHyphen)
                .alias("sub")
                .about("Substitutes templates using structured data. \
                 The idea is to build a tree of data that is used to substitute in various templates, using multiple inputs and outputs.\
                 That way, secrets (like credentials) can be extracted from the vault just once and used wherever needed without them touching disk.\
                 Liquid is used as template engine, and it's possible to refer to and inherit from other templates by their file-stem. \
                 Read more on their website at https://shopify.github.io/liquid .")
                .arg(
                    Arg::with_name("engine")
                        .set(ArgSettings::RequireEquals)
                        .required(false)
                        .multiple(false)
                        .takes_value(true)
                        .long("engine")
                        .value_name("name")
                        .short("e")
                        .default_value("liquid")
                        .possible_values(&["handlebars", "liquid"])
                        .help("The choice of engine used for the substitution. Valid values are 'handlebars' and \
                       'liquid'. \
                       'liquid', the default, is coming with batteries included and very good at handling
                       one template at a time.
                       'handlebars' supports referencing other templates using partials, which \
                       is useful for sharing of common functionality.")
                )
                .arg(
                    Arg::with_name("separator")
                        .set(ArgSettings::RequireEquals)
                        .required(false)
                        .multiple(false)
                        .takes_value(true)
                        .long("separator")
                        .short("s")
                        .default_value("\n")
                        .value_name("separator")
                        .help("The string to use to separate multiple documents that are written to the same stream. \
                            This can be useful to output a multi-document YAML file from multiple input templates \
                            to stdout if the separator is '---'. \
                            The separator is also used when writing multiple templates into the same file, like in 'a:out b:out'.")
                )
                .arg(
                    Arg::with_name("replace")
                        .set(ArgSettings::RequireEquals)
                        .long("replace")
                        .takes_value(true)
                        .value_name("find-this:replace-with-that")
                        .required(false)
                        .multiple(true)
                        .use_delimiter(true)
                        .value_delimiter(":")
                        .help("A simple find & replace for values for the string data to be placed into the template. \
                       The word to find is the first specified argument, the second one is the word to replace it with, \
                       e.g. -r=foo:bar.")
                )
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
    }

    pub mod completions {
        use clap::{App, Arg};
        use std::env;

        lazy_static! {
            static ref SHELL: Result<String, env::VarError> = env::var("SHELL");
        }

        pub fn new<'a, 'b>() -> App<'a, 'b> {
            App::new("completions")
                .about("generate completions for supported shell")
                .arg({
                    let arg = Arg::with_name("shell").required(SHELL.is_err()).help(
                        "The name of the shell, or the path to the shell as exposed by the \
                         $SHELL variable.",
                    );
                    if let Ok(shell) = SHELL.as_ref() {
                        arg.default_value(shell)
                    } else {
                        arg
                    }
                })
        }
    }
}

/// taken from share-secrets-safely/tools
mod parse {
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
    pub mod completions {
        use clap::{App, ArgMatches, Shell};
        use failure::{err_msg, Error, ResultExt};
        use std::{io::stdout, path::Path, str::FromStr};

        pub fn generate(mut app: App, args: &ArgMatches) -> Result<(), Error> {
            let shell = args
                .value_of("shell")
                .ok_or_else(|| err_msg("expected 'shell' argument"))
                .map(|s| {
                    Path::new(s)
                        .file_name()
                        .map(|f| {
                            f.to_str()
                                .expect("os-string to str conversion to work for filename")
                        })
                        .unwrap_or(s)
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
    }

    pub mod process {
        use super::optional_args_with_value;
        use atty;
        use clap::{value_t, ArgMatches};
        use failure::{bail, Error};
        use sheesy_tools::process::{reduce, Command, OutputMode};

        use std::{io::stdout, path::PathBuf};

        pub fn execute(args: &ArgMatches) -> Result<(), Error> {
            let cmds = context_from(args)?;

            let sout = stdout();
            let mut lock = sout.lock();
            reduce(cmds, None, &mut lock).map(|_| ())
        }

        pub fn context_from(args: &ArgMatches) -> Result<Vec<Command>, Error> {
            Ok({
                let mut has_seen_merge_stdin = false;
                let mut cmds = match (args.values_of_os("file"), args.indices_of("file")) {
                    (Some(v), Some(i)) => v
                        .map(|v| {
                            if v == "-" {
                                has_seen_merge_stdin = true;
                                Command::MergeStdin
                            } else {
                                Command::MergePath(PathBuf::from(v))
                            }
                        })
                        .zip(i)
                        .collect(),
                    (None, None) => Vec::new(),
                    _ => unreachable!("expecting clap to work"),
                };

                let select_cmds = optional_args_with_value(args, "pointer", |s| {
                    Command::SelectToBuffer(s.to_owned())
                });
                cmds.extend(select_cmds.into_iter());

                cmds.sort_by_key(|&(_, index)| index);
                let mut cmds: Vec<_> = cmds.into_iter().map(|(c, _)| c).collect();

                if let Ok(output_mode) = value_t!(args, "output", OutputMode) {
                    cmds.insert(0, Command::SetOutputMode(output_mode));
                }

                if atty::isnt(atty::Stream::Stdin) && !has_seen_merge_stdin {
                    let at_position = cmds
                        .iter()
                        .position(|cmd| match *cmd {
                            Command::MergePath(_) | Command::SelectToBuffer(_) => true,
                            _ => false,
                        })
                        .unwrap_or_else(|| cmds.len());
                    cmds.insert(at_position, Command::MergeStdin)
                }
                cmds.push(Command::SerializeBuffer);

                if !cmds.iter().any(|c| match *c {
                    Command::MergeStdin | Command::MergePath(_) => true,
                    _ => false,
                }) {
                    bail!("Please provide structured data from standard input or from a file.");
                }
                cmds
            })
        }
    }

    pub mod substitute {
        use clap::ArgMatches;
        use failure::{bail, Error};
        use itertools::Itertools;
        use sheesy_tools::substitute::{Engine, Spec, StreamOrPath};

        use super::required_os_arg;
        use sheesy_tools::substitute::substitute;
        use std::ffi::OsString;

        #[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct Context {
            pub validate: bool,
            pub replacements: Vec<(String, String)>,
            pub separator: OsString,
            pub engine: Engine,
            pub data: StreamOrPath,
            pub specs: Vec<Spec>,
        }

        pub fn context_from(args: &ArgMatches) -> Result<Context, Error> {
            Ok(Context {
                replacements: {
                    let replace_cmds = args
                        .values_of("replace")
                        .map_or_else(Vec::new, |v| v.map(|s| s.to_owned()).collect());
                    if replace_cmds.len() % 2 != 0 {
                        bail!("Please provide --replace-value arguments in pairs of two. First the value to find, second the one to replace it with");
                    }
                    replace_cmds.into_iter().tuples().collect()
                },
                separator: required_os_arg(args, "separator")?,
                engine: args.value_of("engine").expect("clap to work").parse()?,
                validate: args.is_present("validate"),
                data: args
                    .value_of_os("data")
                    .map_or(StreamOrPath::Stream, Into::into),
                specs: match args.values_of("spec") {
                    Some(v) => v.map(Spec::from).collect(),
                    None => Vec::new(),
                },
            })
        }

        pub fn execute(args: &ArgMatches) -> Result<(), Error> {
            let context = context_from(args)?;
            substitute(
                context.engine,
                &context.data,
                &context.specs,
                &context.separator,
                context.validate,
                &context.replacements,
            )
        }
    }
}

const PROGRAM_NAME: &str = "mcp";
use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version, App, AppSettings,
    ArgMatches,
};

fn main() {
    let app: App = app_from_crate!()
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
