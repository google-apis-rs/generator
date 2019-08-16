use discovery_parser::DiscoveryRestDesc;
use google_rest_api_generator as generator;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    ::env_logger::builder()
        .default_format_timestamp_nanos(true)
        .init();
    let desc: DiscoveryRestDesc =
        reqwest::get("https://www.googleapis.com/discovery/v1/apis/compute/v1/rest")?
            .error_for_status()?
            .json()?;
    let project_name = format!("google_{}_{}", &desc.name, &desc.version);
    generator::generate(
        &project_name,
        &desc,
        "/tmp",
        &std::env::args().nth(1).unwrap(),
    )?;
    Ok(())
}
