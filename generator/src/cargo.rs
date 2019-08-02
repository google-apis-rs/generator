use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug,Clone,Serialize)]
pub(crate) struct Manifest {
    package: Package,
    dependencies: BTreeMap<String, Dependency>,
}

#[derive(Debug,Clone,Serialize)]
pub(crate) struct Package {
    name: String,
    version: String, 
    authors: Vec<String>,
    edition: Option<String>,
}

#[derive(Debug,Clone,Serialize)]
pub(crate) struct Dependency {
    version: Option<String>,
    features: Vec<String>,
    path: Option<String>,
}

impl Dependency {
    fn new(version: Option<&str>, features: &[&str], path: Option<&str>) -> Self {
        Dependency{
            version: version.map(std::borrow::ToOwned::to_owned),
            features: features.into_iter().map(|&x| x.to_owned()).collect(),
            path: path.map(std::borrow::ToOwned::to_owned),
        }
    }
}

pub(crate) fn manifest(crate_name: impl Into<String>) -> Manifest {
    Manifest{
        package: Package{
            name: crate_name.into(),
            version: "0.1.0".to_owned(),
            authors: vec!["Glenn Griffin <ggriffiniii@gmail.com".to_owned()],
            edition: Some("2018".to_owned()),
        },
        dependencies: vec![
            ("serde", Dependency::new(Some("1"), &["derive"], None)),
        ].into_iter().map(|(name, def)| (name.to_owned(), def)).collect(),
    }
}