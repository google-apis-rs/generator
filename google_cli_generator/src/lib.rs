use google_rest_api_generator::Metadata as ApiMetadata;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct Metadata {
    pub git_hash: String,
    pub ymd_date: String,
}

impl Default for Metadata {
    fn default() -> Self {
        Metadata {
            git_hash: env!("GIT_HASH").into(),
            ymd_date: env!("BUILD_DATE").into(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Default)]
pub struct CombinedMetadata {
    pub cli_generator: Metadata,
    pub api_generator: ApiMetadata,
}

pub mod all;
mod cargo;
pub mod cli;
mod util;
