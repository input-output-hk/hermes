//! Manifest resource type implementation.
//! This type defines only serde logic and does not implement `ResourceTrait` trait
//! directly.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer};

use super::{fs::FsResource, uri::Uri, ResourceTrait};

/// Manifest resource definition with the `serde::Deserialize` implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ManifestResource {
    /// File system resource.
    Fs(FsResource),
}

impl ManifestResource {
    /// Upload resource to the file system, returning its path.
    pub(crate) fn upload_to_fs(&self) -> PathBuf {
        match self {
            Self::Fs(fs) => fs.get_path(),
        }
    }

    /// Update current resource to make it relative to the given path.
    pub(crate) fn make_relative_to<P: AsRef<Path>>(&mut self, to: P) {
        match self {
            Self::Fs(fs) => fs.make_relative_to(to),
        }
    }

    /// Get `ResourceTrait` obj.
    pub(crate) fn resource(&self) -> &impl ResourceTrait {
        match self {
            Self::Fs(fs) => fs,
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
                    Ok(Self::Fs(FsResource::new(path)))
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
                    Ok(Self::Fs(FsResource::new(path)))
                } else {
                    Err(anyhow::anyhow!("Empty path in URI"))
                }
            },
            Some(schema) => Err(anyhow::anyhow!("Unsupported URI schema {schema}")),
        }
    }
}

impl<'de> Deserialize<'de> for ManifestResource {
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
        let resource = ManifestResource::from_uri(uri).expect("Cannot create resource from uri");
        assert_eq!(resource, ManifestResource::Fs(FsResource::new("file.txt")));

        let uri = Uri {
            schema: Some("file".to_string()),
            host: None,
            path: Some("file.txt".to_string()),
        };
        let resource = ManifestResource::from_uri(uri).expect("Cannot create resource from uri");
        assert_eq!(resource, ManifestResource::Fs(FsResource::new("file.txt")));

        let uri = Uri {
            schema: Some("file".to_string()),
            host: Some("www.google.com".to_string()),
            path: Some("file.txt".to_string()),
        };
        assert!(ManifestResource::from_uri(uri).is_err());

        let uri = Uri {
            schema: None,
            host: None,
            path: None,
        };
        assert!(ManifestResource::from_uri(uri).is_err());
    }
}
