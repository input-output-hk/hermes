//! Resources module functionality.

// cspell: words splitn

use std::{
    fmt::{Debug, Display},
    io::Read,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Deserializer};

/// URI resource definition.
/// This definition mainly based on the [URI RFC](https://tools.ietf.org/html/rfc3986),
/// but the implementation is not compliant with it and conforms with our needs.
/// The parsing pattern is as follows:
/// `[schema] :// [host] / [path]`
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Uri {
    /// URI schema component.
    schema: Option<String>,
    /// URI host component.
    host: Option<String>,
    /// URI path component.
    path: Option<String>,
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

/// Resource not found error.
#[derive(thiserror::Error, Debug)]
#[error("Resource not found at {0}")]
pub(crate) struct ResourceNotFoundError(String);

/// Cannot get directory content error.
#[derive(thiserror::Error, Debug)]
#[error("Cannot get directory content at {0}")]
pub(crate) struct CannotGetDirectoryContent(String);

/// Resource definition.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Resource {
    /// File system resource.
    FsPath(PathBuf),
}

impl<P: AsRef<Path>> From<P> for Resource {
    fn from(path: P) -> Self {
        Self::FsPath(path.as_ref().to_path_buf())
    }
}

impl Display for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Resource::FsPath(path) => write!(f, "{}", path.display()),
        }
    }
}

impl Resource {
    /// Create resource from URI.
    fn from_uri(uri: Uri) -> anyhow::Result<Self> {
        match uri.schema {
            None => {
                if uri.host.is_some() {
                    return Err(anyhow::anyhow!(
                        "URI with host is not supported for this type of schema",
                    ));
                }
                if let Some(path) = uri.path {
                    Ok(Resource::FsPath(PathBuf::from(path)))
                } else {
                    Err(anyhow::anyhow!("Empty path in URI"))
                }
            },
            Some(schema) if schema == "file" => {
                if uri.host.is_some() {
                    return Err(anyhow::anyhow!(
                        "URI with host is not supported for this type of schema",
                    ));
                }
                if let Some(path) = uri.path {
                    Ok(Resource::FsPath(PathBuf::from(path)))
                } else {
                    Err(anyhow::anyhow!("Empty path in URI"))
                }
            },
            Some(schema) => Err(anyhow::anyhow!("Unsupported URI schema {schema}")),
        }
    }

    /// Get resource name.
    pub(crate) fn name(&self) -> anyhow::Result<String> {
        match self {
            Self::FsPath(path) => {
                Ok(path
                    .file_name()
                    .ok_or(anyhow::anyhow!("cannot get path name"))?
                    .to_str()
                    .ok_or(anyhow::anyhow!("cannot convert path name to str"))?
                    .to_string())
            },
        }
    }

    /// Make resource relative to given path.
    pub(crate) fn make_relative_to<P: AsRef<Path>>(&mut self, to: P) {
        match self {
            Resource::FsPath(path) => {
                if path.is_relative() {
                    *path = to.as_ref().join(&path);
                }
            },
        }
    }

    /// Get data reader for the resource.
    pub(crate) fn get_reader(&self) -> anyhow::Result<impl Read + Debug> {
        match self {
            Resource::FsPath(path) => {
                std::fs::File::open(path).map_err(|err| {
                    if err.kind() == std::io::ErrorKind::NotFound {
                        ResourceNotFoundError(self.to_string()).into()
                    } else {
                        err.into()
                    }
                })
            },
        }
    }

    /// Get directory content.
    pub(crate) fn get_directory_content(&self) -> anyhow::Result<std::fs::ReadDir> {
        match self {
            Resource::FsPath(path) => {
                std::fs::read_dir(path).map_err(|err| {
                    if err.kind() == std::io::ErrorKind::NotFound {
                        CannotGetDirectoryContent(self.to_string()).into()
                    } else {
                        err.into()
                    }
                })
            },
        }
    }
}

impl<'de> Deserialize<'de> for Resource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        let uri = Uri::parse_from_str(&s);
        Self::from_uri(uri).map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;

    #[test]
    fn uri_parsing_test() {
        assert_eq!(
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

    #[test]
    fn resource_from_uri_test() {
        let uri = Uri {
            schema: None,
            host: None,
            path: Some("file.txt".to_string()),
        };
        let resource = Resource::from_uri(uri).expect("Cannot create resource from uri");
        assert_eq!(resource, Resource::FsPath("file.txt".into()));

        let uri = Uri {
            schema: Some("file".to_string()),
            host: None,
            path: Some("file.txt".to_string()),
        };
        let resource = Resource::from_uri(uri).expect("Cannot create resource from uri");
        assert_eq!(resource, Resource::FsPath("file.txt".into()));

        let uri = Uri {
            schema: Some("file".to_string()),
            host: Some("www.google.com".to_string()),
            path: Some("file.txt".to_string()),
        };
        assert!(Resource::from_uri(uri).is_err());

        let uri = Uri {
            schema: None,
            host: None,
            path: None,
        };
        assert!(Resource::from_uri(uri).is_err());
    }

    #[test]
    fn resource_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let file_name = "file.txt";
        std::fs::write(dir.path().join(file_name), [0, 1, 2])
            .expect("Cannot write data to file.txt");

        let mut resource = Resource::FsPath(file_name.into());

        let err = resource.get_reader().expect_err("Should return error");
        assert!(err.is::<ResourceNotFoundError>());

        resource.make_relative_to(dir.path());
        resource.get_reader().expect("Cannot get reader");
    }
}
