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
    /// Application WASM Modules.
    pub(crate) modules: Vec<ManifestModule>,
    /// Path to the www directory.
    pub(crate) www: Option<Resource>,
    /// Path to the share directory.
    pub(crate) share: Option<Resource>,
}

/// `Manifest` `modules` item field definition.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ManifestModule {
    /// Path to the WASM module package file.
    pub(crate) file: Resource,
    /// Application WASM module name.
    pub(crate) name: Option<String>,
    /// Path to the WASM module config JSON file.
    pub(crate) config: Option<Resource>,
    /// Path to the WASM module share directory.
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

        if manifest.www.is_none() && manifest.share.is_none() && manifest.modules.is_empty() {
            anyhow::bail!(
                "Invalid manifest, must contain at least one module or www or share directory"
            );
        }

        let dir_path = path
            .parent()
            .ok_or_else(|| FileError::from_path(path, None))?;
        manifest.metadata.make_relative_to(dir_path);
        manifest.modules.iter_mut().for_each(|m| {
            m.file.make_relative_to(dir_path);
            if let Some(config) = m.config.as_mut() {
                config.make_relative_to(dir_path);
            }
            if let Some(share) = m.share.as_mut() {
                share.make_relative_to(dir_path);
            }
        });
        if let Some(www) = manifest.www.as_mut() {
            www.make_relative_to(dir_path);
        }
        if let Some(share) = manifest.share.as_mut() {
            share.make_relative_to(dir_path);
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
        #[serde(default)]
        modules: Vec<ManifestModuleSerde>,
        www: Option<Resource>,
        share: Option<Resource>,
    }

    #[derive(Deserialize)]
    struct ManifestModuleSerde {
        file: Resource,
        name: Option<String>,
        config: Option<Resource>,
        share: Option<Resource>,
    }

    impl From<ManifestSerde> for super::Manifest {
        fn from(def: ManifestSerde) -> Self {
            Self {
                name: def.name,
                metadata: def.metadata,
                modules: def
                    .modules
                    .into_iter()
                    .map(|der| {
                        super::ManifestModule {
                            file: der.file,
                            name: der.name,
                            config: der.config,
                            share: der.share,
                        }
                    })
                    .collect(),
                www: def.www,
                share: def.share,
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
                    "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_app_manifest.schema.json",
                    "name": "app_name",
                    "metadata": "metadata.json",
                    "modules": [{
                        "file": "module.hmod",
                        "name": "module_name",
                        "config": "config.json",
                        "share": "share"
                    }],
                    "www": "www",
                    "share": "share"
                }).to_string();
            std::fs::write(&path, manifest_json_data).expect("Cannot create manifest.json file");
            let manifest = Manifest::from_file(&path).expect("Cannot create manifest");
            assert_eq!(manifest, Manifest {
                name: "app_name".to_string(),
                metadata: Resource::Fs(FsResource::new(dir_path.join("metadata.json"))),
                modules: vec![ManifestModule {
                    file: Resource::Fs(FsResource::new(dir_path.join("module.hmod"))),
                    name: Some("module_name".to_string()),
                    config: Some(Resource::Fs(FsResource::new(dir_path.join("config.json")))),
                    share: Some(Resource::Fs(FsResource::new(dir_path.join("share")))),
                }],
                www: Some(Resource::Fs(FsResource::new(dir_path.join("www")))),
                share: Some(Resource::Fs(FsResource::new(dir_path.join("share")))),
            });
        }

        {
            let path = dir_path.join("manifest.json");
            let manifest_json_data = serde_json::json!({
                    "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_app_manifest.schema.json",
                    "name": "app_name",
                    "metadata": "/metadata.json",
                    "modules": [{
                        "file": "/module.hmod",
                        "name": "module_name",
                        "config": "/config.json",
                        "share": "/share"
                    }],
                    "www": "/www",
                    "share": "/share"
                }).to_string();
            std::fs::write(&path, manifest_json_data).expect("Cannot create manifest.json file");
            let manifest = Manifest::from_file(&path).expect("Cannot create manifest");
            assert_eq!(manifest, Manifest {
                name: "app_name".to_string(),
                metadata: Resource::Fs(FsResource::new("/metadata.json")),
                modules: vec![ManifestModule {
                    file: Resource::Fs(FsResource::new("/module.hmod")),
                    name: Some("module_name".to_string()),
                    config: Some(Resource::Fs(FsResource::new("/config.json"))),
                    share: Some(Resource::Fs(FsResource::new("/share"))),
                }],
                www: Some(Resource::Fs(FsResource::new("/www"))),
                share: Some(Resource::Fs(FsResource::new("/share"))),
            });
        }

        {
            let path = dir_path.join("manifest.json");
            let manifest_json_data = serde_json::json!({
                    "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_app_manifest.schema.json",
                }).to_string();
            std::fs::write(&path, manifest_json_data).expect("Cannot create manifest.json file");
            assert!(Manifest::from_file(&path).is_err());
        }
    }
}
