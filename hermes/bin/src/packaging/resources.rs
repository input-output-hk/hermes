//! Resources module functionality.

use std::{
    fmt::{Debug, Display},
    io::Read,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Deserializer};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Uri {
    schema: Option<String>,
    host: Option<String>,
    path: String,
}

impl Uri {
    #[allow(clippy::indexing_slicing)]
    pub(crate) fn parse_from_str(s: &str) -> Self {
        let schema_and_host_and_path = s.splitn(2, "://").collect::<Vec<_>>();

        let (schema, host_and_path) = if schema_and_host_and_path.len() == 2 {
            let schema = schema_and_host_and_path[0].to_string();
            let host_and_path = schema_and_host_and_path[1]
                .splitn(2, '/')
                .collect::<Vec<_>>();
            (Some(schema), host_and_path)
        } else {
            let host_and_path = schema_and_host_and_path[0]
                .splitn(2, '/')
                .collect::<Vec<_>>();
            (None, host_and_path)
        };

        let (host, path) = if host_and_path.len() == 2 {
            let host = host_and_path[0].to_string();
            let path = host_and_path[1].to_string();
            match host.as_str() {
                "" => (None, format!("/{path}")),
                "." | ".." | "~" => (None, format!("{host}/{path}")),
                _ => (Some(host), path),
            }
        } else {
            let path = host_and_path[0].to_string();
            (None, path)
        };

        Self { schema, host, path }
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

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Resource {
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

    pub(crate) fn make_relative_to<P: AsRef<Path>>(&mut self, to: P) {
        match self {
            Resource::FsPath(path) => {
                if path.is_relative() {
                    *path = to.as_ref().join(&path);
                }
            },
        }
    }

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

        match uri.schema {
            None => {
                if uri.host.is_some() {
                    return Err(serde::de::Error::custom(
                        "URI with host is not supported for this type of schema",
                    ));
                }
                Ok(Resource::FsPath(PathBuf::from(uri.path)))
            },
            Some(schema) if schema == "file" => {
                if uri.host.is_some() {
                    return Err(serde::de::Error::custom(
                        "URI with host is not supported for this type of schema",
                    ));
                }
                Ok(Resource::FsPath(PathBuf::from(uri.path)))
            },
            Some(schema) => {
                Err(serde::de::Error::custom(format!(
                    "Unsupported URI schema {schema}"
                )))
            },
        }
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
                path: "file.txt".to_string()
            }
        );
        assert_eq!(Uri::parse_from_str("www.google.com/file.txt"), Uri {
            schema: None,
            host: Some("www.google.com".to_string()),
            path: "file.txt".to_string()
        });
        assert_eq!(Uri::parse_from_str("file://file.txt"), Uri {
            schema: Some("file".to_string()),
            host: None,
            path: "file.txt".to_string()
        });
        assert_eq!(Uri::parse_from_str("file://../file.txt"), Uri {
            schema: Some("file".to_string()),
            host: None,
            path: "../file.txt".to_string()
        });
        assert_eq!(Uri::parse_from_str("file://~/file.txt"), Uri {
            schema: Some("file".to_string()),
            host: None,
            path: "~/file.txt".to_string()
        });
        assert_eq!(Uri::parse_from_str("file:///file.txt"), Uri {
            schema: Some("file".to_string()),
            host: None,
            path: "/file.txt".to_string()
        });
        assert_eq!(Uri::parse_from_str("file.txt"), Uri {
            schema: None,
            host: None,
            path: "file.txt".to_string()
        });
        assert_eq!(Uri::parse_from_str("/file.txt"), Uri {
            schema: None,
            host: None,
            path: "/file.txt".to_string()
        });
        assert_eq!(Uri::parse_from_str("./file.txt"), Uri {
            schema: None,
            host: None,
            path: "./file.txt".to_string()
        });
        assert_eq!(Uri::parse_from_str("~/file.txt"), Uri {
            schema: None,
            host: None,
            path: "~/file.txt".to_string()
        });
        assert_eq!(Uri::parse_from_str("../file.txt"), Uri {
            schema: None,
            host: None,
            path: "../file.txt".to_string()
        });
        assert_eq!(Uri::parse_from_str(""), Uri {
            schema: None,
            host: None,
            path: String::new()
        });
    }

    #[test]
    fn resource_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let file_name = "file.txt";
        std::fs::write(dir.path().join(file_name), [0, 1, 2])
            .expect("Cannot write data to file.txt");

        let mut resource = serde_json::from_value::<Resource>(serde_json::json!(file_name))
            .expect("Cannot parse ResourceLocation json");

        let err = resource.get_reader().expect_err("Should return error");
        assert!(err.is::<ResourceNotFoundError>());

        resource.make_relative_to(dir.path());
        resource.get_reader().expect("Cannot get reader");
    }
}
