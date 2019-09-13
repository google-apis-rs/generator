use discovery_parser::DiscoveryRestDesc;
use google_rest_api_generator as generator;
use std::error::Error;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "any_api",
    about = "Generate Rust API bindings for a Google Rest API"
)]
struct Opt {
    #[structopt(long = "name")]
    name: String,
    #[structopt(long = "version")]
    version: String,
    #[structopt(long = "output_dir", default_value = "/tmp", parse(from_os_str))]
    output_dir: PathBuf,
}

/// Alter the URL to generate output for a different API.
/// Otherwise, prefer using the machinery in https://github.com/google-apis-rs/generated to
/// generate any API, CLI and more
fn main() -> Result<(), Box<dyn Error>> {
    ::env_logger::builder()
        .default_format_timestamp_nanos(true)
        .init();
    let opt = Opt::from_args();
    let desc: DiscoveryRestDesc = reqwest::get(&format!(
        "https://www.googleapis.com/discovery/v1/apis/{}/{}/rest",
        &opt.name, &opt.version
    ))?
    .error_for_status()?
    .json()?;
    let project_name = format!("google_{}_{}", &desc.name, &desc.version);
    let project_dir = opt.output_dir.join(&project_name);
    println!("Writing to {:?}", &project_dir);
    generator::generate(&project_dir, &desc)?;
    Ok(())
}
