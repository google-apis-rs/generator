use discovery_parser::DiscoveryRestDesc;
use reqwest;
use serde::Deserialize;
use std::collections::BTreeSet;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut all_param_types = BTreeSet::new();
    for_each_api(|rest_desc| {
        for param in rest_desc.parameters.values() {
            all_param_types.insert((param.typ.clone(), param.format.clone()));
        }
        for resource in rest_desc.resources.values() {
            for method in resource.methods.values() {
                for param in method.parameters.values() {
                    all_param_types.insert((param.typ.clone(), param.format.clone()));
                }
            }
        }
    })?;
    println!("Param types: {:?}", all_param_types);
    Ok(())
}

fn for_each_api<F>(mut f: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnMut(&DiscoveryRestDesc),
{
    let all_apis: ApiList = reqwest::get("https://www.googleapis.com/discovery/v1/apis")?.json()?;
    println!("There are {} apis", all_apis.items.len());
    for api in all_apis.items {
        match get_api(&api.discovery_rest_url) {
            Ok(rest_desc) => f(&rest_desc),
            Err(err) => eprintln!("Failed to get {}: {}", &api.discovery_rest_url, err),
        }
    }
    Ok(())
}

fn get_api(url: &str) -> Result<DiscoveryRestDesc, Box<dyn std::error::Error>> {
        println!("Fetching {}", url);
        Ok(reqwest::get(url)?.json()?)
}