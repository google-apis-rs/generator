use std::error::Error;
use google_rest_api_generator as generator;

fn main() -> Result<(), Box<dyn Error>> {
    ::env_logger::builder().default_format_timestamp_nanos(true).init();
    generator::generate(
        "https://www.googleapis.com/discovery/v1/apis/oauth2/v2/rest",
        "/tmp",
        &std::env::args().nth(1).unwrap(),
    )?;
    Ok(())
}
