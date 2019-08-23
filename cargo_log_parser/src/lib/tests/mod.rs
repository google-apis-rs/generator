fn assert_complete_error<T: std::fmt::Debug, E: std::fmt::Debug>(res: Result<T, nom::Err<E>>) {
    panic!(
        "Not complete: {}",
        match res {
            Ok(t) => format!("{:?}", t),
            Err(nom::Err::Failure(_)) | Err(nom::Err::Error(_)) => return,
            Err(e) => format!("{:?}", e),
        }
    )
}

fn assert_incomplete_error<T: std::fmt::Debug, E: std::fmt::Debug>(res: Result<T, nom::Err<E>>) {
    panic!(
        "Not incomplete: {}",
        match res {
            Ok(t) => format!("{:?}", t),
            Err(nom::Err::Incomplete(_)) => return,
            Err(e) => format!("{:?}", e),
        }
    )
}

mod quoted {
    use super::super::quoted_name;
    use super::assert_incomplete_error;
    #[test]
    fn it_works_on_valid_input() {
        assert_eq!(
            quoted_name(b"`hello-there1`"),
            Ok((&b""[..], &b"hello-there1"[..]))
        );
    }
    #[test]
    fn fails_on_partial_input() {
        assert_incomplete_error(quoted_name(b"`hello-"))
    }
}

mod line {
    use crate::line_without_ending;
    use crate::tests::assert_incomplete_error;

    #[test]
    fn it_succeeds_on_valid_input() {
        assert_eq!(
            line_without_ending(b"foo\n").unwrap(),
            (&b""[..], &b"foo"[..])
        );
    }

    #[test]
    fn it_needs_a_complete_line() {
        assert_incomplete_error(line_without_ending(b"foo"))
    }
}

mod parse_errors {
    use crate::{parse_errors, CrateWithError};
    static CARGO_ERRORS: &[u8] = include_bytes!("./fixtures/check-with-error.log");
    static CARGO_ERRORS_PARALLEL: &[u8] =
        include_bytes!("./fixtures/check-with-error-parallel.log");

    #[test]
    fn it_succeeds_on_valid_sequential_input() {
        assert_eq!(
            parse_errors(CARGO_ERRORS).unwrap(),
            (
                &b""[..],
                vec![
                    CrateWithError { name: "!".into() },
                    CrateWithError {
                        name: "google-urlshortener1".into()
                    }
                ]
            )
        );
    }
    #[test]
    fn it_succeeds_on_valid_parallel_input() {
        assert_eq!(
            parse_errors(CARGO_ERRORS_PARALLEL).unwrap(),
            (
                &b""[..],
                vec![
                    CrateWithError {
                        name: "google-groupsmigration1".into()
                    },
                    CrateWithError {
                        name: "google-oauth2".into()
                    },
                    CrateWithError {
                        name: "google-pagespeedonline5".into()
                    }
                ]
            )
        );
    }
}

mod error_line {
    use super::super::line_with_error;
    use crate::tests::{assert_complete_error, assert_incomplete_error};
    use crate::CrateWithError;

    #[test]
    fn it_succeeds_and_parses_the_correct_crate_name_on_valid_input() {
        assert_eq!(
            line_with_error(&b"error: Could not compile `google-groupsmigration1`.\n"[..]),
            Ok((
                &b""[..],
                CrateWithError {
                    name: "google-groupsmigration1".into()
                }
            ))
        );
        assert_incomplete_error(line_with_error(
            &b"error: Could not compile `google-groupsmigration1`"[..],
        ));
        assert_incomplete_error(line_with_error(&b"error: Could not "[..]));
        assert_incomplete_error(line_with_error(&b"err"[..]));
    }

    #[test]
    fn it_fails_on_invalid_input() {
        assert_complete_error(line_with_error(
            b"    Checking google-videointelligence1_p3beta1 v0.1.0 (/Users/some/lib)\n",
        ));

        assert_incomplete_error(line_with_error(
            b"    Checking google-videointelligence1_p3beta1 v0.1.0 (/Users/s",
        ));
    }
}
