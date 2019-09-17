use crate::util::concat_with_and;
use discovery_parser::DiscoveryRestDesc;
use google_rest_api_generator::{APIDesc, Method as ApiMethod, Resource as ApiResource};
use serde::Serialize;
use shared::Api;
use std::{collections::VecDeque, iter::FromIterator};

const APP_IDENT: &str = "app";

#[derive(Serialize)]
pub struct Model {
    /// The name of the top-level app identifier
    app_ident: String,
    /// The name of the crate for 'use ' statement
    lib_crate_name_for_use: String,
    /// The name of the CLI program
    program_name: String,
    /// The full semantic version of the CLI
    cli_version: String,
    /// A one-line summary of what the API does
    description: String,
    /// A list of resources, along with their capabilities, with methods or without
    resources: Vec<Resource>,
}

impl Model {
    pub fn new(api: Api, desc: &DiscoveryRestDesc, api_desc: &APIDesc) -> Self {
        Model {
            app_ident: APP_IDENT.to_owned(),
            lib_crate_name_for_use: api.lib_crate_name.replace('-', "_"),
            program_name: api.bin_name,
            cli_version: api.cli_crate_version.expect("available cli crate version"),
            description: desc.description.clone(),
            resources: {
                let mut deque = VecDeque::from_iter(api_desc.resources.iter());
                let mut resources = Vec::new();
                while let Some(api_resource) = deque.pop_front() {
                    resources.push(Resource::from(api_resource));
                    deque.extend(api_resource.resources.iter());
                }
                resources
            },
        }
    }
}

#[derive(Serialize)]
struct Resource {
    parent_ident: String,
    ident: String,
    name: String,
    about: String,
    methods: Vec<Method>,
}

impl From<&ApiResource> for Resource {
    fn from(r: &ApiResource) -> Self {
        let name = r.ident.to_string();
        let parent_count = r.parent_path.segments.len().saturating_sub(2); // skip top-level resources module

        Resource {
            parent_ident: if parent_count == 0 {
                APP_IDENT.into()
            } else {
                format!(
                    "{}{}",
                    r.parent_path
                        .segments
                        .last()
                        .expect("at least one item")
                        .ident
                        .to_string(),
                    parent_count.saturating_sub(1)
                )
            },
            ident: format!("{}{}", name, parent_count),
            name,
            about: if r.methods.is_empty() {
                "a resource with sub-resources".into()
            } else {
                format!(
                    "methods: {}",
                    concat_with_and(r.methods.iter().map(|m| m.ident.to_string()))
                )
            },
            methods: r.methods.iter().map(Method::from).collect(),
        }
    }
}

#[derive(Serialize)]
struct Method {
    name: String,
    about: Option<String>,
}

impl From<&ApiMethod> for Method {
    fn from(m: &ApiMethod) -> Self {
        Method {
            name: m.ident.to_string(),
            about: m.description.clone(),
        }
    }
}
