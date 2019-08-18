use structopt::StructOpt;

const PROGRAM_NAME: &str = "mcp";

/// taken from share-secrets-safely/tools
mod options;
/// taken from share-secrets-safely/tools
mod parse;

mod cmds {
    pub mod fetch_specs {
        use crate::options::fetch_specs::Args;
        use failure::Error;

        pub fn execute(
            Args {
                discovery_json_path: _,
                output_directory: _,
            }: Args,
        ) -> Result<(), Error> {
            unimplemented!("fetch specs")
        }
    }

    pub mod completions;
}

use options::Args;
use options::SubCommand::*;

fn main() {
    let args: Args = Args::from_args();

    let res = match args.cmd {
        Completions(args) => cmds::completions::generate(Args::clap(), args),
        FetchSpecs(args) => cmds::fetch_specs::execute(args),
    };
    failure_tools::ok_or_exit(res);
}
