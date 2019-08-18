use structopt::StructOpt;

const PROGRAM_NAME: &str = "mcp";

mod cmds;
/// taken from share-secrets-safely/tools
mod options;

use options::Args;
use options::SubCommand::*;

fn main() {
    let args: Args = Args::from_args();

    let res = match args.cmd {
        Completions(args) => cmds::completions::generate(Args::clap(), args),
        FetchSpecs(args) => cmds::fetch_specs::execute(args),
        Substitute(_args) => unimplemented!(),
    };
    failure_tools::ok_or_exit(res);
}
