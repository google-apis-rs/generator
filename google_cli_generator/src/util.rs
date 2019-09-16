pub fn concat_with_and(items: impl Iterator<Item = impl AsRef<str>>) -> String {
    let items = items.map(|s| s.as_ref().to_owned()).collect::<Vec<_>>();
    if items.is_empty() {
        return String::new();
    }
    let mut buf = items[..items.len() - 1].join(", ");
    if items.len() > 1 {
        buf.push_str(" and ");
    }
    if items.len() > 0 {
        buf.push_str(items.last().expect("last element"));
    }
    buf
}

#[cfg(test)]
mod tests {
    mod concat_with_and {
        use super::super::concat_with_and;
        #[test]
        fn empty_input_yields_empty_string() {
            assert_eq!(concat_with_and(Vec::<&str>::new().iter()), "");
        }

        #[test]
        fn single_input_yields_item() {
            assert_eq!(concat_with_and(["foo"].iter()), "foo");
        }

        #[test]
        fn two_input_yields_items_connected_with_and() {
            assert_eq!(concat_with_and(["foo", "bar"].iter()), "foo and bar");
        }

        #[test]
        fn multiple_input_yields_last_two_items_connected_with_and_the_others_with_comma() {
            assert_eq!(
                concat_with_and(["foo", "bar", "baz"].iter()),
                "foo, bar and baz"
            );
        }
    }
}
