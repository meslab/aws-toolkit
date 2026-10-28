pub(crate) fn matches_filters(name: &str, include: &[String], exclude: &[String]) -> bool {
    let included = include.is_empty() || include.iter().any(|x| name.contains(x));

    included && exclude.iter().all(|x| !name.contains(x))
}

pub(crate) fn sanitize_string(input: &str) -> String {
    input
        .chars()
        .fold(
            (String::with_capacity(input.len()), false),
            |(mut acc, last_was_dash), c| {
                if c.is_alphanumeric() {
                    acc.push(c);
                    (acc, false)
                } else if !last_was_dash {
                    acc.push('-');
                    (acc, true)
                } else {
                    (acc, last_was_dash)
                }
            },
        )
        .0
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{matches_filters, sanitize_string};

    #[test]
    fn test_sanitise_string() {
        let input = "G!-test-- ---long:bow;";
        let value = sanitize_string(input);
        assert_eq!(value, "G-test-long-bow");
    }

    #[test]
    fn test_matches_filters_with_empty_include_matches_everything() {
        assert!(matches_filters("anything", &[], &[]));
    }

    #[test]
    fn test_matches_filters_requires_include_match() {
        let include = vec!["prod".to_string()];
        assert!(matches_filters("my-prod-bucket", &include, &[]));
        assert!(!matches_filters("my-dev-bucket", &include, &[]));
    }

    #[test]
    fn test_matches_filters_exclude_wins() {
        let include = vec!["prod".to_string()];
        let exclude = vec!["legacy".to_string()];
        assert!(!matches_filters("prod-legacy-bucket", &include, &exclude));
    }
}
