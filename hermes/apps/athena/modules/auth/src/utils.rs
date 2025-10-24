//! Utilities functions

/// Macro to extract headers with optional prefix filtering
#[macro_export]
macro_rules! extract_header {
    // Extract header without prefix
    ($headers:expr, $name:expr) => {
        $headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case($name))
            .and_then(|(_, values)| values.first().cloned())
    };

    // Extract header with prefix and strip it
    ($headers:expr, $name:expr, $prefix:expr) => {
        $headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case($name))
            .and_then(|(_, values)| values.first().cloned())
            .filter(|value| value.starts_with($prefix))
            .and_then(|value| value.strip_prefix($prefix).map(|s| s.to_string()))
    };
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    #[test_case(
        vec![
            ("content-type".to_string(), vec!["application/json".to_string()]),
            ("authorization".to_string(), vec!["Bearer catid.123@test.com/signature".to_string()]),
        ],
        "authorization",
        Some("Bearer catid.123@test.com/signature".to_string())
        ; "extract header without prefix"
    )]
    fn test_extract_header_without_prefix(
        headers: Vec<(String, Vec<String>)>,
        header_name: &str,
        expected: Option<String>,
    ) {
        let result = extract_header!(headers, header_name);
        assert_eq!(result, expected);
    }

    #[test_case(
        vec![
            ("content-type".to_string(), vec!["application/json".to_string()]),
            ("authorization".to_string(), vec!["Bearer catid.123@test.com/signature".to_string()]),
        ],
        "authorization",
        "Bearer ",
        Some("catid.123@test.com/signature".to_string())
        ; "extract header with Bearer prefix - should strip the prefix"
    )]
    #[test_case(
        vec![
            ("content-type".to_string(), vec!["application/json".to_string()]),
            ("authorization".to_string(), vec!["Basic dXNlcjpwYXNz".to_string()]),
        ],
        "authorization",
        "Bearer ",
        None
        ; "extract header with Bearer prefix when header has Basic auth"
    )]
    #[test_case(
        vec![("Authorization".to_string(), vec!["Bearer catid.123@test.com/signature".to_string()])],
        "authorization",
        "Bearer ",
        Some("catid.123@test.com/signature".to_string())
        ; "case insensitive header name matching"
    )]
    #[test_case(
        vec![("content-type".to_string(), vec!["application/json".to_string()])],
        "authorization",
        "Bearer ",
        None
        ; "extract non-existent header"
    )]
    #[test_case(
        vec![("authorization".to_string(), vec![
            "Bearer catid.123@test.com/signature".to_string(),
            "Bearer catid.456@test.com/signature2".to_string(),
        ])],
        "authorization",
        "Bearer ",
        Some("catid.123@test.com/signature".to_string())
        ; "extract header with multiple values - should return first match"
    )]
    fn test_extract_header_with_prefix(
        headers: Vec<(String, Vec<String>)>,
        header_name: &str,
        prefix: &str,
        expected: Option<String>,
    ) {
        let result = extract_header!(headers, header_name, prefix);
        assert_eq!(result, expected);
    }
}
