use discovery_parser::DiscoveryRestDesc;
use google_rest_api_generator as generator;
use std::error::Error;
use std::path::Path;

/// Alter the URL to generate output for a different API.
/// Otherwise, prefer using the machinery in https://github.com/google-apis-rs/generated to
/// generate any API, CLI and more
fn main() -> Result<(), Box<dyn Error>> {
    ::env_logger::builder()
        .default_format_timestamp_nanos(true)
        .init();
    let desc: DiscoveryRestDesc =
        reqwest::get("https://www.googleapis.com/discovery/v1/apis/admin/directory_v1/rest")?
            .error_for_status()?
            .json()?;
    let project_name = format!("google_{}_{}", &desc.name, &desc.version);
    generator::generate(&project_name, &desc, Path::new("/tmp").join(&project_name))?;
    Ok(())
}
