//! This module, in some way or form, should contain all logic used to generate names.
//! These must be reused throughout the library.
//! You will find all/most of the constants here.
use discovery_parser::{
    generated::{ApiIndexV1, Icons, Item, Kind},
    DiscoveryRestDesc,
};
use failure::{bail, format_err, Error};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, path::Path, path::PathBuf};

/// A bunch of constants which must be the single source for constants
/// that are not API specific.
#[derive(Serialize, Deserialize)]
pub struct Standard {
    /// A cargo project relative path to the manifest file
    pub cargo_toml_path: String,
    /// A project relative path to the Rust library implementation
    pub lib_path: String,
    /// The name of the folder into which we want to generate the library projects
    pub lib_dir: String,
    /// The name of the folder containing specification files, as seen from the 'generated' repository
    pub spec_dir: String,
}

impl Default for Standard {
    fn default() -> Self {
        Standard {
            cargo_toml_path: "Cargo.toml".into(),
            lib_path: "src/lib.rs".into(),
            lib_dir: "lib".into(),
            spec_dir: "etc/api".into(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct MappedIndex {
    pub standard: Standard,
    pub api: Vec<Api>,
}

#[derive(Serialize, Deserialize)]
pub struct Api {
    /// The sanitized API name. See `sanitized_name(...)` for more information
    pub name: String,
    /// The official API id, good for identification
    pub id: String,
    /// A 'gen' directory relative path to the project manifest
    pub lib_cargo_file: PathBuf,
    /// A 'gen' directory relative path to the file containing any kind of generation or build error
    pub error_file: PathBuf,
    /// A 'gen' directory relative path into which all files pertaining the API must be placed
    pub gen_dir: PathBuf,
    /// A 'gen' directory relative path to the google discovery specification file
    pub spec_file: PathBuf,
    /// A suitable name for the crate
    pub crate_name: String,
    /// A suitable name for being a target in 'make'
    pub make_target: String,
    /// The URL to the google discovery specification
    pub rest_url: String,
}

impl TryFrom<Item> for Api {
    type Error = Error;

    fn try_from(value: Item) -> Result<Self, Self::Error> {
        let name = sanitized_name(&value.name).into();
        let gen_dir = PathBuf::from(&name).join(&value.version);
        let standard = Standard::default();
        Ok(Api {
            spec_file: gen_dir.join("spec.json"),
            id: value.id,
            lib_cargo_file: gen_dir
                .join(standard.lib_dir)
                .join(standard.cargo_toml_path),
            error_file: gen_dir.join("generator-errors.log"),
            gen_dir,
            name,
            rest_url: value.discovery_rest_url,
            crate_name: crate_name(&value.name, &value.version)?,
            make_target: make_target(&value.name, &value.version)?,
        })
    }
}

impl TryFrom<&DiscoveryRestDesc> for Api {
    type Error = Error;

    fn try_from(value: &DiscoveryRestDesc) -> Result<Self, Self::Error> {
        let item = Item {
            kind: Kind::DiscoveryDirectoryItem,
            id: value.id.to_owned(),
            name: value.name.to_owned(),
            version: value.version.to_owned(),
            title: value.title.to_owned(),
            description: value.description.to_owned(),
            discovery_rest_url: "<unset>".into(),
            icons: Icons {
                x16: "".to_string(),
                x32: "".to_string(),
            },
            documentation_link: None,
            preferred: false,
            discovery_link: None,
            labels: None,
        };
        Api::try_from(item)
    }
}

impl MappedIndex {
    pub fn validated(mut self, output_directory: &Path) -> Self {
        let ci_whitelist = ["admin:directory_v1", "compute:v1", "drive:v3", "oauth2:v2"];
        let info = ci_info::get();
        if info.ci {
            info!(
                "Running on CI '{:?}' - limiting APIs to {whitelist:?}",
                info.vendor,
                whitelist = ci_whitelist
            );
        }
        self.api.retain(|api| {
            let spec_path = output_directory.join(&api.spec_file);
            let is_allowed = if info.ci {
                ci_whitelist.contains(&api.id.as_str())
            } else {
                true
            };
            if !is_allowed {
                return false;
            }
            if !spec_path.is_file() {
                error!(
                    "Dropping API '{}' as its spec file at '{}' does not exist",
                    api.crate_name,
                    spec_path.display(),
                );
                return false
            }
            let error_log = output_directory.join(&api.error_file);
            if error_log.is_file() {
                error!("Dropping API '{}' as it previously failed with generator errors, see '{}' for details.", api.crate_name, error_log.display());
                return false;
            }
            true
        });
        self
    }
    pub fn from_index(index: ApiIndexV1) -> Result<Self, Error> {
        Ok(MappedIndex {
            standard: Standard::default(),
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

/// Currently does the following
/// * strip off all numbers from the tail, until the first non-digit is found
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

/// Normalize the version string to adhere to a standard format.
/// The latter could certainly be expressed here at some point, right
/// now it's implied by the tests.
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
