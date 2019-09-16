use crate::util::concat_with_and;
use discovery_parser::DiscoveryRestDesc;
use google_rest_api_generator::{APIDesc, Method as ApiMethod, Resource as ApiResource};
use serde::Serialize;
use shared::Api;

#[derive(Serialize)]
pub struct Model {
    /// The name of the crate for 'use ' statement
    lib_crate_name_for_use: String,
    /// The name of the CLI program
    program_name: String,
    /// The full semantic version of the CLI
    cli_version: String,
    /// A one-line summary of what the API does
    description: String,
    /// A list of resources, along with their capabilities
    resources: Vec<Resource>,
}

impl Model {
    pub fn new(api: Api, desc: &DiscoveryRestDesc, api_desc: &APIDesc) -> Self {
        Model {
            lib_crate_name_for_use: api.lib_crate_name.replace('-', "_"),
            program_name: api.bin_name,
            cli_version: api.cli_crate_version.expect("available cli crate version"),
            description: desc.description.clone(),
            resources: api_desc.resources.iter().map(Resource::from).collect(),
        }
    }
}

#[derive(Serialize)]
struct Resource {
    name: String,
    about: String,
    methods: Vec<Method>,
}

impl From<&ApiResource> for Resource {
    fn from(r: &ApiResource) -> Self {
        assert!(
            r.resources.is_empty(),
            "currently there is no support for nested resources"
        );
        assert!(
            !r.methods.is_empty(),
            "there should at least be one method per resource"
        );

        Resource {
            name: r.ident.to_string(),
            about: format!(
                "methods: {}",
                concat_with_and(r.methods.iter().map(|m| m.ident.to_string()))
            ),
            methods: r.methods.iter().map(Method::from).collect(),
        }
    }
}

#[derive(Serialize)]
struct Method {
    name: String,
}

impl From<&ApiMethod> for Method {
    fn from(m: &ApiMethod) -> Self {
        Method {
            name: m.ident.to_string(),
        }
    }
}
