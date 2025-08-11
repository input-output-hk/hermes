//! Generally used utility functions.

/// Parse a path string into a vector of path elements applying the `/` and `\`
/// delimiters.
pub(crate) fn parse_path(path: &str) -> Vec<String> {
    path.split(&['/', '\\'])
        .map(ToString::to_string)
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;

    #[test]
    fn parse_path_test() {
        assert!(parse_path("").is_empty());

        assert!(parse_path("/").is_empty());
        assert_eq!(parse_path("/a"), vec!["a".to_string()]);
        assert_eq!(parse_path("/a/b"), vec!["a".to_string(), "b".to_string()]);

        assert!(parse_path(r"\").is_empty());
        assert_eq!(parse_path(r"\a"), vec!["a".to_string()]);
        assert_eq!(parse_path(r"\a\b"), vec!["a".to_string(), "b".to_string()]);

        assert_eq!(parse_path("//a//b"), vec!["a".to_string(), "b".to_string()]);
        assert_eq!(parse_path(r"\\a\\b"), vec![
            "a".to_string(),
            "b".to_string()
        ]);
    }
}
