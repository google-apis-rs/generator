use discovery_parser::{normalize, RawDiscoveryRestDesc, RestDescOrErr};
use serde::Deserialize;
use std::error::Error;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiList {
    items: Vec<ApiSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiSpec {
    name: String,
    discovery_rest_url: String,
}

#[test]
fn successfully_parse_all_apis() -> Result<(), Box<dyn Error>> {
    let mut errors = 0;
    let mut successes = 0;
    let all_apis: ApiList = reqwest::get("https://www.googleapis.com/discovery/v1/apis")?.json()?;
    for api in &all_apis.items {
        println!("Fetching {}", api.discovery_rest_url);
        let res: Result<RestDescOrErr, _> = reqwest::get(&api.discovery_rest_url)?.json();
        match res {
            Ok(RestDescOrErr::RestDesc(desc)) => {
                successes += 1;
                println!("{:#?}", desc);
            }
            Ok(RestDescOrErr::Err(err)) => {
                eprintln!("{}: {:?}", api.discovery_rest_url, err);
                errors += 1;
            }
            Err(err) => {
                eprintln!("{}: json error: {:?}", api.discovery_rest_url, err);
                errors += 1;
            }
        }
    }
    println!("success: {}, errors: {}", successes, errors);
    Ok(())
}

#[test]
fn parse_one_api() -> Result<(), Box<dyn Error>> {
    let url = "https://cloudresourcemanager.googleapis.com/$discovery/rest?version=v1";
    println!("Fetching {}", url);
    let body: String = reqwest::get(url)?.text()?;
    std::fs::write("/tmp/content", &body)?;
    let desc: RawDiscoveryRestDesc = serde_json::from_str(&body)?;
    let desc = normalize(desc);
    println!("{:#?}", desc);
    Ok(())
}
