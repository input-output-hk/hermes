//! Resource builder type implementation.
//! This type defines only serde logic and does not implement `ResourceTrait` trait
//! directly.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer};

use super::{fs::FsResource, uri::Uri, ResourceTrait};

/// Resource builder definition with the `serde::Deserialize` implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResourceBuilder {
    /// File system resource.
    Fs(PathBuf),
}

impl ResourceBuilder {
    /// Upload resource to the file system, returning its path.
    pub(crate) fn upload_to_fs(&self) -> PathBuf {
        match self {
            Self::Fs(fs) => fs.clone(),
        }
    }

    /// Update current resource to make it relative to the given path.
    pub(crate) fn make_relative_to<P: AsRef<Path>>(&mut self, to: P) {
        match self {
            Self::Fs(fs) => {
                if fs.is_relative() {
                    *fs = to.as_ref().join(&fs);
                }
            },
        }
    }

    /// Get `ResourceTrait` obj.
    pub(crate) fn build(&self) -> impl ResourceTrait {
        match self {
            Self::Fs(fs) => FsResource::new(fs),
        }
    }

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
                    Ok(Self::Fs(path.into()))
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
                    Ok(Self::Fs(path.into()))
                } else {
                    Err(anyhow::anyhow!("Empty path in URI"))
                }
            },
            Some(schema) => Err(anyhow::anyhow!("Unsupported URI schema {schema}")),
        }
    }
}

impl<'de> Deserialize<'de> for ResourceBuilder {
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
        let resource = ResourceBuilder::from_uri(uri).expect("Cannot create resource from uri");
        assert_eq!(resource, ResourceBuilder::Fs("file.txt".into()));

        let uri = Uri {
            schema: Some("file".to_string()),
            host: None,
            path: Some("file.txt".to_string()),
        };
        let resource = ResourceBuilder::from_uri(uri).expect("Cannot create resource from uri");
        assert_eq!(resource, ResourceBuilder::Fs("file.txt".into()));

        let uri = Uri {
            schema: Some("file".to_string()),
            host: Some("www.google.com".to_string()),
            path: Some("file.txt".to_string()),
        };
        assert!(ResourceBuilder::from_uri(uri).is_err());

        let uri = Uri {
            schema: None,
            host: None,
            path: None,
        };
        assert!(ResourceBuilder::from_uri(uri).is_err());
    }
}
