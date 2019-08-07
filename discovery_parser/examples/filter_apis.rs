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

fn count_resources<'a>(
    resources: impl Iterator<Item = &'a discovery_parser::ResourceDesc>,
) -> usize {
    resources
        .map(|resource| {
            let sub_resources: usize = count_resources(resource.resources.values());
            1 + sub_resources
        })
        .sum()
}

fn count_methods<'a>(resources: impl Iterator<Item = &'a discovery_parser::ResourceDesc>) -> usize {
    resources
        .map(|resource| {
            let sub_methods: usize = count_methods(resource.resources.values());
            resource.methods.len() + sub_methods
        })
        .sum()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for_each_api(|rest_desc| {
        for_each_resource(rest_desc, |resource| {
            for method in resource.methods.values() {
                for (id, param) in &method.parameters {
                    if param.location == "path" {
                        println!(
                            "{}: {} {} {}",
                            id,
                            &param.typ,
                            param.format.as_ref().map(|x| x.as_str()).unwrap_or(""),
                            if param.enumeration.is_empty() {
                                ""
                            } else {
                                "enum"
                            }
                        );
                    }
                }
            }
        });
    })?;
    Ok(())
}

fn for_each_resource<F>(rest_desc: &DiscoveryRestDesc, mut f: F)
where
    F: FnMut(&discovery_parser::ResourceDesc),
{
    fn per_resource<F>(res: &discovery_parser::ResourceDesc, f: &mut F)
    where
        F: FnMut(&discovery_parser::ResourceDesc),
    {
        for sub_resource in res.resources.values() {
            per_resource(sub_resource, f);
        }
        f(res)
    }

    for resource in rest_desc.resources.values() {
        per_resource(resource, &mut f);
    }
}

fn for_each_api<F>(mut f: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnMut(&DiscoveryRestDesc),
{
    let client = reqwest::Client::new();
    let all_apis: ApiList = client
        .get("https://www.googleapis.com/discovery/v1/apis")
        .send()?
        .json()?;
    println!("There are {} apis", all_apis.items.len());
    for api in all_apis.items {
        match get_api(&client, &api.discovery_rest_url) {
            Ok(rest_desc) => f(&rest_desc),
            Err(err) => eprintln!("Failed to get {}: {}", &api.discovery_rest_url, err),
        }
    }
    Ok(())
}

fn get_api(
    client: &reqwest::Client,
    url: &str,
) -> Result<DiscoveryRestDesc, Box<dyn std::error::Error>> {
    eprintln!("Fetching {}", url);
    Ok(client.get(url).send()?.json()?)
}
