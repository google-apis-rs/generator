//! This module, in some way or form, should contain all logic used to generate names.
//! These must be reused throughout the library.
use discovery_parser::generated::{ApiIndexV1, Item};
use failure::{bail, format_err, Error};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct MappedIndex {
    pub api: Vec<Api>,
}

#[derive(Serialize, Deserialize)]
pub struct Api {
    pub name: String,
    pub gen_dir: PathBuf,
    pub spec_file: PathBuf,
    pub crate_name: String,
    pub make_target: String,
    pub rest_url: String,
}

impl TryFrom<Item> for Api {
    type Error = Error;

    fn try_from(value: Item) -> Result<Self, Self::Error> {
        let name = sanitized_name(&value.name).into();
        let gen_dir = PathBuf::from(&name).join(&value.version);
        Ok(Api {
            spec_file: gen_dir.join("spec.json"),
            gen_dir,
            name,
            rest_url: value.discovery_rest_url,
            crate_name: crate_name(&value.name, &value.version)?,
            make_target: make_target(&value.name, &value.version)?,
        })
    }
}

impl MappedIndex {
    pub fn from_index(index: ApiIndexV1) -> Result<Self, Error> {
        Ok(MappedIndex {
            api: index
                .items
                .into_iter()
                .map(Api::try_from)
                .collect::<Result<Vec<_>, Error>>()?,
        })
    }
}

pub fn crate_name(name: &str, version: &str) -> Result<String, Error> {
    make_target(name, version).map(|n| format!("google-{}", n))
}

pub fn sanitized_name(name: &str) -> &str {
    if let Some(pos) = name.rfind(|c| !char::is_digit(c, 10)) {
        &name[..=pos]
    } else {
        name
    }
}

pub fn make_target(name: &str, version: &str) -> Result<String, Error> {
    Ok(format!(
        "{name}{version}",
        name = sanitized_name(name),
        version = parse_version(version)?
    ))
}

pub fn parse_version(version: &str) -> Result<String, Error> {
    let inner = |version: &str| {
        if version.len() < 2 {
            bail!("version string too small");
        }
        if !version.is_ascii() {
            bail!("can only handle ascii versions");
        }
        if version == "alpha" || version == "beta" {
            return Ok(version.into());
        }

        fn transform_version(version: &str) -> Result<String, Error> {
            let mut bytes = version.bytes();
            if bytes.next() != Some(b'v') {
                bail!("A version must start with 'v'");
            }
            let mut out = String::new();
            let mut separator = Some('_');
            for b in bytes {
                let c = match b {
                    b'.' => b'd',
                    b @ b'0'..=b'9' => b,
                    b @ b'a'..=b'z' => {
                        if let Some(sep) = separator.take() {
                            out.push(sep);
                        }
                        b
                    }
                    b => bail!("unexpected character '{}'", b),
                } as char;
                out.push(c);
            }
            Ok(out)
        }

        let mut tokens = version.splitn(2, '_');
        if let (Some(left), Some(right)) = (tokens.next(), tokens.next()) {
            return Ok(format!(
                "{version}_{name}",
                version = transform_version(right)?,
                name = left
            ));
        }
        transform_version(version)
    };
    inner(version).map_err(|e| format_err!("invalid version '{}': {}", version, e))
}

#[cfg(test)]
mod tests;
