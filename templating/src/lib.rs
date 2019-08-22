#![forbid(unsafe_code)]

#[macro_use]
extern crate failure;
extern crate serde_json as json;
extern crate serde_yaml as yaml;

extern crate atty;
extern crate base64;
extern crate handlebars;
extern crate liquid;
extern crate liquid_error;
extern crate yaml_rust;

pub mod substitute;
