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
    use super::sanitize_string;

    #[test]
    fn test_sanitise_string() {
        let input = "G!-test-- ---long:bow;";
        let value = sanitize_string(input);
        assert_eq!(value, "G-test-long-bow");
    }
}
