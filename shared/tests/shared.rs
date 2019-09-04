mod sanitized_name {
    use shared::sanitized_name;

    #[test]
    fn it_does_not_alter_anything_else() {
        assert_eq!(sanitized_name("2foo"), "2foo");
        assert_eq!(sanitized_name("fo2oo"), "fo2oo");
        assert_eq!(sanitized_name("foo"), "foo");
    }
    #[test]
    fn it_strips_numbers_off_the_tail() {
        // specifically for adexchangebuyer , actually
        assert_eq!(sanitized_name("foo2"), "foo");
        assert_eq!(sanitized_name("foo20"), "foo")
    }
}

mod crate_name {
    use shared::crate_name;

    #[test]
    fn it_produces_a_valid_crate_name() {
        assert_eq!(crate_name("youtube", "v2.0").unwrap(), "google-youtube2d0")
    }
}

mod make_target {
    use shared::make_target;

    #[test]
    fn it_produces_a_valid_make_target() {
        assert_eq!(make_target("youtube", "v1.3").unwrap(), "youtube1d3")
    }
}

mod parse_version {
    use insta::assert_snapshot;
    use itertools::Itertools;
    use shared::parse_version;
    use std::io::{BufRead, BufReader};

    const KNOWN_VERSIONS: &str = include_str!("./fixtures/known-versions");

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
        assert_snapshot!(expected);
    }
}
