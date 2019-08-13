use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    ::pretty_env_logger::init_timed();
    generator::generate(
        "https://www.googleapis.com/discovery/v1/apis/compute/v1/rest",
        "/tmp",
        &std::env::args().nth(1).unwrap(),
    )?;
    Ok(())
}
