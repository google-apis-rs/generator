use discovery_parser::DiscoveryRestDesc;
use std::error::Error;

#[test]
fn parse_one_api() -> Result<(), Box<dyn Error>> {
    let url = "https://www.googleapis.com/discovery/v1/apis/admin/directory_v1/rest";
    println!("Fetching {}", url);
    let body: String = reqwest::get(url)?.text()?;
    std::fs::write("/tmp/content", &body)?;
    let desc: DiscoveryRestDesc = serde_json::from_str(&body)?;
    println!("{:#?}", desc);
    Ok(())
}
