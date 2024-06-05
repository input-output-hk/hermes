//! Resources module functionality.

// cspell: words splitn

pub(crate) mod bytes_resource;
pub(crate) mod fs_resource;
mod uri;

use std::{
    fmt::{Debug, Display},
    io::Read,
    path::Path,
};

use fs_resource::FsResource;
use serde::{Deserialize, Deserializer};
use uri::Uri;

/// Resource trait definition.
pub(crate) trait ResourceTrait {
    /// Get resource name.
    fn name(&self) -> anyhow::Result<String>;

    /// Check if resource is a directory.
    fn is_dir(&self) -> bool;

    /// Check if resource is a file.
    fn is_file(&self) -> bool;

    /// Get data reader for the resource.
    fn get_reader(&self) -> anyhow::Result<impl Read + Debug>;

    /// Get directory content.
    fn get_directory_content(&self) -> anyhow::Result<Vec<Self>>
    where Self: Sized;
}

/// Resource definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Resource {
    /// File system resource.
    Fs(FsResource),
}

impl Display for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fs(fs) => Display::fmt(fs, f),
        }
    }
}

impl ResourceTrait for Resource {
    fn name(&self) -> anyhow::Result<String> {
        match self {
            Self::Fs(fs) => fs.name(),
        }
    }

    fn is_dir(&self) -> bool {
        match self {
            Self::Fs(fs) => fs.is_dir(),
        }
    }

    fn is_file(&self) -> bool {
        match self {
            Self::Fs(fs) => fs.is_file(),
        }
    }

    fn get_reader(&self) -> anyhow::Result<impl Read + Debug> {
        match self {
            Self::Fs(fs) => fs.get_reader(),
        }
    }

    fn get_directory_content(&self) -> anyhow::Result<Vec<Self>>
    where Self: Sized {
        match self {
            Self::Fs(fs) => {
                Ok(fs
                    .get_directory_content()?
                    .into_iter()
                    .map(Self::Fs)
                    .collect())
            },
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
                    Ok(Resource::Fs(FsResource::new(path)))
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
                    Ok(Resource::Fs(FsResource::new(path)))
                } else {
                    Err(anyhow::anyhow!("Empty path in URI"))
                }
            },
            Some(schema) => Err(anyhow::anyhow!("Unsupported URI schema {schema}")),
        }
    }

    /// Update current resource to make it relative to the given path.
    pub(crate) fn make_relative_to<P: AsRef<Path>>(&mut self, to: P) {
        match self {
            Resource::Fs(fs) => fs.make_relative_to(to),
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
    use super::*;

    #[test]
    fn resource_from_uri_test() {
        let uri = Uri {
            schema: None,
            host: None,
            path: Some("file.txt".to_string()),
        };
        let resource = Resource::from_uri(uri).expect("Cannot create resource from uri");
        assert_eq!(resource, Resource::Fs(FsResource::new("file.txt")));

        let uri = Uri {
            schema: Some("file".to_string()),
            host: None,
            path: Some("file.txt".to_string()),
        };
        let resource = Resource::from_uri(uri).expect("Cannot create resource from uri");
        assert_eq!(resource, Resource::Fs(FsResource::new("file.txt")));

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
