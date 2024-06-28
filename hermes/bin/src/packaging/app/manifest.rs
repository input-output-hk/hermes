//! Hermes application package manifest.json struct.

use std::path::Path;

use crate::packaging::{
    resources::{fs_resource::FsResource, Resource},
    schema_validation::SchemaValidator,
    FileError,
};

/// Hermes application package manifest.json definition.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Manifest {
    /// Package name.
    pub(crate) name: String,
    /// Path to the metadata JSON file.
    pub(crate) metadata: Resource,
    /// Definition of the srv directory.
    pub(crate) srv: Option<ManifestSrv>,
}

/// `Manifest` `srv` field definition.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ManifestSrv {
    /// Path to the www directory.
    pub(crate) www: Option<Resource>,
    /// Path to the share directory.
    pub(crate) share: Option<Resource>,
}

impl Manifest {
    /// WASM module manifest.json schema.
    const MANIFEST_SCHEMA: &'static str =
        include_str!("../../../../schemas/hermes_app_manifest.schema.json");

    /// Default package name.
    fn default_package_name() -> String {
        String::from("app")
    }

    /// Default metadata.json file path.
    fn default_metadata_path() -> Resource {
        Resource::Fs(FsResource::new("metadata.json"))
    }

    /// Create a `Manifest` from a path.
    pub(crate) fn from_file<P: AsRef<Path>>(path_to_manifest: P) -> anyhow::Result<Self> {
        let path = path_to_manifest.as_ref();
        let file = std::fs::File::open(path).map_err(|_| FileError::from_path(path, None))?;

        let schema_validator = SchemaValidator::from_str(Self::MANIFEST_SCHEMA)?;
        let mut manifest: Manifest = schema_validator
            .deserialize_and_validate::<_, serde_def::ManifestSerde>(file)
            .map_err(|err| FileError::from_path(path, Some(err)))?
            .into();

        let dir_path = path
            .parent()
            .ok_or_else(|| FileError::from_path(path, None))?;
        manifest.metadata.make_relative_to(dir_path);
        if let Some(srv) = manifest.srv.as_mut() {
            if let Some(www) = srv.www.as_mut() {
                www.make_relative_to(dir_path);
            }
            if let Some(share) = srv.share.as_mut() {
                share.make_relative_to(dir_path);
            }
        }

        Ok(manifest)
    }
}

#[allow(missing_docs, clippy::missing_docs_in_private_items)]
mod serde_def {
    //! Serde definition of the manifest objects.

    use serde::Deserialize;

    use crate::packaging::resources::Resource;

    #[derive(Deserialize)]
    pub(crate) struct ManifestSerde {
        #[serde(default = "super::Manifest::default_package_name")]
        name: String,
        #[serde(default = "super::Manifest::default_metadata_path")]
        metadata: Resource,
        srv: Option<ManifestSrvSerde>,
    }

    #[derive(Deserialize)]
    struct ManifestSrvSerde {
        www: Option<Resource>,
        share: Option<Resource>,
    }

    impl From<ManifestSerde> for super::Manifest {
        fn from(def: ManifestSerde) -> Self {
            Self {
                name: def.name,
                metadata: def.metadata,
                srv: def.srv.map(|der| {
                    super::ManifestSrv {
                        www: der.www,
                        share: der.share,
                    }
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;

    #[test]
    fn manifest_from_file_test() {
        let dir = TempDir::new().expect("Failed to create temp dir.");
        let dir_path = dir.path();

        {
            let path = dir_path.join("manifest.json");
            let manifest_json_data = serde_json::json!({
                    "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_module_manifest.schema.json",
                    "name": "app_name",
                    "metadata": "metadata.json",
                    "srv": {
                        "www": "www",
                        "share": "share"
                    }
                }).to_string();
            std::fs::write(&path, manifest_json_data).expect("Cannot create manifest.json file");
            let manifest = Manifest::from_file(&path).expect("Cannot create manifest");
            assert_eq!(manifest, Manifest {
                name: "app_name".to_string(),
                metadata: Resource::Fs(FsResource::new(dir_path.join("metadata.json"))),
                srv: Some(ManifestSrv {
                    www: Some(Resource::Fs(FsResource::new(dir_path.join("www")))),
                    share: Some(Resource::Fs(FsResource::new(dir_path.join("share")))),
                }),
            });
        }
    }
}
