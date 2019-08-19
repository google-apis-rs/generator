//! This module, in some way or form, should contain all logic used to generate names.
//! These must be reused throughout the library.
use failure::{bail, format_err, Error};

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
    fn inner(version: &str) -> Result<String, Error> {
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
    }
    inner(version).map_err(|e| format_err!("invalid version '{}': {}", version, e))
}

#[cfg(test)]
mod tests {
    mod sanitized_name {
        use crate::shared::sanitized_name;

        #[test]
        fn it_does_not_alter_anything_else() {
            assert_eq!(sanitized_name("2foo"), "2foo");
            assert_eq!(sanitized_name("fo2oo"), "fo2oo");
            assert_eq!(sanitized_name("foo"), "foo");
        }
        #[test]
        fn it_strips_numbers_off_the_tail() {
            assert_eq!(sanitized_name("foo2"), "foo");
            assert_eq!(sanitized_name("foo20"), "foo")
        }
    }
    mod crate_name {
        use crate::shared::crate_name;

        #[test]
        fn it_produces_a_valid_crate_name() {
            assert_eq!(crate_name("youtube", "v2.0").unwrap(), "google-youtube2d0")
        }
    }
    mod make_target {
        use crate::shared::make_target;

        #[test]
        fn it_produces_a_valid_make_target() {
            assert_eq!(make_target("youtube", "v1.3").unwrap(), "youtube1d3")
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
