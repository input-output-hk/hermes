//! Resources module functionality.

// cspell: words splitn

mod fs_resource;
mod uri;

use std::{
    fmt::{Debug, Display},
    io::Read,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Deserializer};
use uri::Uri;

/// Resource not found error.
#[derive(thiserror::Error, Debug)]
#[error("Resource not found at {0}")]
pub(crate) struct ResourceNotFoundError(String);

/// Cannot get directory content error.
#[derive(thiserror::Error, Debug)]
#[error("Cannot get directory content at {0}")]
pub(crate) struct CannotGetDirectoryContent(String);

/// `Resource` trait definition.
#[allow(dead_code)]
pub(crate) trait ResourceTrait {
    /// Get resource name.
    fn name(&self) -> anyhow::Result<String>;

    /// Check if resource is a directory.
    fn is_dir(&self) -> bool;

    /// Check if resource is a file.
    fn is_file(&self) -> bool;

    /// Make resource relative to given path.
    fn make_relative_to<P: AsRef<Path>>(&mut self, to: P)
    where Self: Sized;

    /// Get data reader for the resource.
    fn get_reader(&self) -> anyhow::Result<impl Read + Debug>
    where Self: Sized;

    /// Get directory content.
    fn get_directory_content(&self) -> anyhow::Result<Vec<Self>>
    where Self: Sized;
}

/// Resource definition.
#[derive(Debug, Clone, PartialEq, Eq)]
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

    /// Check if resource is a directory.
    pub(crate) fn is_dir(&self) -> bool {
        match self {
            Resource::FsPath(path) => path.is_dir(),
        }
    }

    /// Check if resource is a file.
    pub(crate) fn is_file(&self) -> bool {
        match self {
            Resource::FsPath(path) => path.is_file(),
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
    pub(crate) fn get_directory_content(&self) -> anyhow::Result<Vec<Resource>> {
        let mut res = Vec::new();
        match self {
            Resource::FsPath(path) => {
                let entries = std::fs::read_dir(path).map_err(|err| {
                    if err.kind() == std::io::ErrorKind::NotFound {
                        anyhow::Error::new(CannotGetDirectoryContent(self.to_string()))
                    } else {
                        anyhow::Error::new(err)
                    }
                })?;
                for entry in entries {
                    res.push(Resource::FsPath(entry?.path()));
                }
            },
        }

        Ok(res)
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

/// Create resource from URI.
fn create_resource_from_uri(uri: Uri) -> anyhow::Result<Box<dyn ResourceTrait>> {
    match uri.schema {
        None => {
            if uri.host.is_some() {
                return Err(anyhow::anyhow!(
                    "URI with host is not supported for this type of schema",
                ));
            }
            if let Some(path) = uri.path {
                Ok(Box::new(PathBuf::from(path)))
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
                Ok(Box::new(PathBuf::from(path)))
            } else {
                Err(anyhow::anyhow!("Empty path in URI"))
            }
        },
        Some(schema) => Err(anyhow::anyhow!("Unsupported URI schema {schema}")),
    }
}

impl<'de> Deserialize<'de> for Box<dyn ResourceTrait> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        let uri = Uri::parse_from_str(&s);
        create_resource_from_uri(uri).map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
