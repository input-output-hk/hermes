//! URI hermes specific parsing implementation.

// cspell: words splitn

/// URI resource definition.
/// This definition mainly based on the [URI RFC](https://tools.ietf.org/html/rfc3986),
/// but the implementation is not compliant with it and conforms with our needs.
/// The parsing pattern is as follows:
/// `[schema] :// [host] / [path]`
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(PartialEq, Eq)]
pub(crate) struct Uri {
    /// URI schema component.
    pub(crate) schema: Option<String>,
    /// URI host component.
    pub(crate) host: Option<String>,
    /// URI path component.
    pub(crate) path: Option<String>,
}

impl Uri {
    /// Parse URI from string with the following pattern:
    /// `[schema] :// [host] / [path]`
    #[allow(clippy::indexing_slicing)]
    pub(crate) fn parse_from_str(s: &str) -> Self {
        let schema_and_host_and_path = s.splitn(2, "://").collect::<Vec<_>>();
        let mut schema = None;
        let mut host = None;
        let mut path = None;

        if schema_and_host_and_path.len() == 2 {
            schema = Some(schema_and_host_and_path[0].to_string());

            let host_and_path = schema_and_host_and_path[1]
                .splitn(2, '/')
                .collect::<Vec<_>>();
            if host_and_path.len() == 2 {
                host = Some(host_and_path[0].to_string());
                path = Some(host_and_path[1].to_string());
            } else {
                host = Some(host_and_path[0].to_string());
            }
        } else {
            path = Some(schema_and_host_and_path[0].to_string());
        }

        Self {
            schema: schema.filter(|s| !s.is_empty()),
            host: host.filter(|s| !s.is_empty()),
            path: path.filter(|s| !s.is_empty()),
        }
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;

    #[test]
    fn uri_parsing_test() {
        debug_assert_eq!(
            Uri::parse_from_str("https://www.google.com/file.txt"),
            Uri {
                schema: Some("https".to_string()),
                host: Some("www.google.com".to_string()),
                path: Some("file.txt".to_string())
            }
        );
        assert_eq!(Uri::parse_from_str("://www.google.com/file.txt"), Uri {
            schema: None,
            host: Some("www.google.com".to_string()),
            path: Some("file.txt".to_string())
        });
        assert_eq!(Uri::parse_from_str("www.google.com/file.txt"), Uri {
            schema: None,
            host: None,
            path: Some("www.google.com/file.txt".to_string()),
        });
        assert_eq!(Uri::parse_from_str("file://www.google.com"), Uri {
            schema: Some("file".to_string()),
            host: Some("www.google.com".to_string()),
            path: None
        });
        assert_eq!(Uri::parse_from_str("file:///../file.txt"), Uri {
            schema: Some("file".to_string()),
            host: None,
            path: Some("../file.txt".to_string())
        });
        assert_eq!(Uri::parse_from_str("file:///~/file.txt"), Uri {
            schema: Some("file".to_string()),
            host: None,
            path: Some("~/file.txt".to_string())
        });
        assert_eq!(Uri::parse_from_str("file:///file.txt"), Uri {
            schema: Some("file".to_string()),
            host: None,
            path: Some("file.txt".to_string())
        });
        assert_eq!(Uri::parse_from_str("file.txt"), Uri {
            schema: None,
            host: None,
            path: Some("file.txt".to_string())
        });
        assert_eq!(Uri::parse_from_str("/file.txt"), Uri {
            schema: None,
            host: None,
            path: Some("/file.txt".to_string())
        });
        assert_eq!(Uri::parse_from_str("./file.txt"), Uri {
            schema: None,
            host: None,
            path: Some("./file.txt".to_string())
        });
        assert_eq!(Uri::parse_from_str("~/file.txt"), Uri {
            schema: None,
            host: None,
            path: Some("~/file.txt".to_string())
        });
        assert_eq!(Uri::parse_from_str("../file.txt"), Uri {
            schema: None,
            host: None,
            path: Some("../file.txt".to_string())
        });
        assert_eq!(Uri::parse_from_str(""), Uri {
            schema: None,
            host: None,
            path: None,
        });
    }
}
