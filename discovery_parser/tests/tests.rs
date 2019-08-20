use discovery_parser::DiscoveryRestDesc;
use std::error::Error;

const API_SPEC: &str = include_str!("./spec.json");

#[test]
fn parse_one_api() -> Result<(), Box<dyn Error>> {
    let desc: DiscoveryRestDesc = serde_json::from_str(API_SPEC)?;
    println!("{:#?}", desc);
    Ok(())
}
