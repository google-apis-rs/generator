//! This module, in some way or form, should contain all logic used to generate names.
//! These must be reused throughout the library.
use failure::Error;

pub fn crate_name(name: &str, version: &str) -> Result<String, Error> {
    make_target(name, version).map(|n| format!("google-{}", n))
}

pub fn make_target(name: &str, version: &str) -> Result<String, Error> {
    unimplemented!()
}

pub fn parse_version(version: &str) -> Result<String, Error> {
    Ok(version.into())
}

#[cfg(test)]
mod tests {
    mod crate_name {
        use crate::shared::crate_name;

        #[test]
        fn it_produces_a_valid_crate_name() {
            assert_eq!(crate_name("youtube", "v1.3").unwrap(), "google-youtube1")
        }
    }
    mod make_target {
        use crate::shared::make_target;

        #[test]
        fn it_produces_a_valid_make_target() {
            assert_eq!(make_target("youtube", "v1.3").unwrap(), "youtube1")
        }
    }
    mod parse_version {
        use super::super::parse_version;
        use insta::assert_snapshot_matches;
        use itertools::Itertools;
        use std::io::{BufRead, BufReader};

        const KNOWN_VERSIONS: &str = include_str!("../../tests/mcp/fixtures/shared/known-versions");

        #[test]
        fn it_works_for_all_known_inputs() {
            let expected = BufReader::new(KNOWN_VERSIONS.as_bytes())
                .lines()
                .filter_map(Result::ok)
                .map(|api_version| {
                    format!(
                        "{input} {output}",
                        input = api_version,
                        output = parse_version(&api_version).unwrap()
                    )
                })
                .join("\n");
            assert_snapshot_matches!(expected);
        }
    }
}
