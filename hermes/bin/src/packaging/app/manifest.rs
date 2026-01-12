//! Hermes application package manifest.json struct.

use std::path::Path;

use super::super::{FileError, schema_validation::SchemaValidator};
use crate::hdf5::resources::ResourceBuilder;

/// Hermes application package manifest.json definition.
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(PartialEq, Eq)]
pub(crate) struct Manifest {
    /// Package name.
    pub(crate) name: String,
    /// Path to the icon svg file.
    pub(crate) icon: ResourceBuilder,
    /// Path to the metadata JSON file.
    pub(crate) metadata: ResourceBuilder,
    /// Application WASM Modules.
    pub(crate) modules: Vec<ManifestModule>,
    /// Path to the www directory.
    pub(crate) www: Option<ResourceBuilder>,
    /// Path to the share directory.
    pub(crate) share: Option<ResourceBuilder>,
}

/// `Manifest` `modules` item field definition.
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(PartialEq, Eq)]
pub(crate) struct ManifestModule {
    /// Path to the WASM module package file.
    pub(crate) package: ResourceBuilder,
    /// Application WASM module name.
    pub(crate) name: Option<String>,
    /// Path to the WASM module config JSON file.
    pub(crate) config: Option<ResourceBuilder>,
    /// Path to the WASM module share directory.
    pub(crate) share: Option<ResourceBuilder>,
}

impl Manifest {
    /// WASM module manifest.json schema.
    const MANIFEST_SCHEMA: &'static str =
        include_str!("../../../../schemas/hermes_app_manifest.schema.json");

    /// Default package name.
    fn default_package_name() -> String {
        String::from("app")
    }

    /// Default icon.svg file path.
    fn default_icon_path() -> ResourceBuilder {
        ResourceBuilder::Fs("icon.svg".into())
    }

    /// Default metadata.json file path.
    fn default_metadata_path() -> ResourceBuilder {
        ResourceBuilder::Fs("metadata.json".into())
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
        manifest.icon.make_relative_to(dir_path);
        manifest.metadata.make_relative_to(dir_path);
        manifest.modules.iter_mut().for_each(|m| {
            m.package.make_relative_to(dir_path);
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

    use crate::hdf5::resources::ResourceBuilder;

    #[derive(Deserialize)]
    pub(crate) struct ManifestSerde {
        #[serde(default = "super::Manifest::default_package_name")]
        name: String,
        #[serde(default = "super::Manifest::default_icon_path")]
        icon: ResourceBuilder,
        #[serde(default = "super::Manifest::default_metadata_path")]
        metadata: ResourceBuilder,
        #[serde(default)]
        modules: Vec<ManifestModuleSerde>,
        www: Option<ResourceBuilder>,
        share: Option<ResourceBuilder>,
    }

    #[derive(Deserialize)]
    struct ManifestModuleSerde {
        package: ResourceBuilder,
        name: Option<String>,
        config: Option<ResourceBuilder>,
        share: Option<ResourceBuilder>,
    }

    impl From<ManifestSerde> for super::Manifest {
        fn from(def: ManifestSerde) -> Self {
            Self {
                name: def.name,
                metadata: def.metadata,
                icon: def.icon,
                modules: def
                    .modules
                    .into_iter()
                    .map(|der| super::ManifestModule {
                        package: der.package,
                        name: der.name,
                        config: der.config,
                        share: der.share,
                    })
                    .collect(),
                www: def.www,
                share: def.share,
            }
        }
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use temp_dir::TempDir;

    use super::*;

    #[test]
    #[allow(clippy::too_many_lines)]
    fn manifest_from_file_test() {
        let dir = TempDir::new().unwrap();
        let dir_path = dir.path();

        {
            let path = dir_path.join("manifest.json");
            let manifest_json_data = serde_json::json!({
                    "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_app_manifest.schema.json",
                    "name": "app_name",
                    "icon": "icon.svg",
                    "metadata": "metadata.json",
                    "modules": [{
                        "package": "module.hmod",
                        "name": "module_name",
                        "config": "config.json",
                        "share": "share"
                    }],
                    "www": "www",
                    "share": "share"
                }).to_string();
            std::fs::write(&path, manifest_json_data).unwrap();
            let manifest = Manifest::from_file(&path).unwrap();
            assert_eq!(
                manifest,
                Manifest {
                    name: "app_name".to_string(),
                    icon: ResourceBuilder::Fs(dir_path.join("icon.svg")),
                    metadata: ResourceBuilder::Fs(dir_path.join("metadata.json")),
                    modules: vec![ManifestModule {
                        package: ResourceBuilder::Fs(dir_path.join("module.hmod")),
                        name: Some("module_name".to_string()),
                        config: Some(ResourceBuilder::Fs(dir_path.join("config.json"))),
                        share: Some(ResourceBuilder::Fs(dir_path.join("share"))),
                    }],
                    www: Some(ResourceBuilder::Fs(dir_path.join("www"))),
                    share: Some(ResourceBuilder::Fs(dir_path.join("share"))),
                }
            );
        }

        {
            let path = dir_path.join("manifest.json");
            let manifest_json_data = serde_json::json!({
                    "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_app_manifest.schema.json",
                    "name": "app_name",
                    "icon": "/icon.svg",
                    "metadata": "/metadata.json",
                    "modules": [{
                        "package": "/module.hmod",
                        "name": "module_name",
                        "config": "/config.json",
                        "share": "/share"
                    }],
                    "www": "/www",
                    "share": "/share"
                }).to_string();
            std::fs::write(&path, manifest_json_data).unwrap();
            let manifest = Manifest::from_file(&path).unwrap();
            assert_eq!(
                manifest,
                Manifest {
                    name: "app_name".to_string(),
                    icon: ResourceBuilder::Fs("/icon.svg".into()),
                    metadata: ResourceBuilder::Fs("/metadata.json".into()),
                    modules: vec![ManifestModule {
                        package: ResourceBuilder::Fs("/module.hmod".into()),
                        name: Some("module_name".to_string()),
                        config: Some(ResourceBuilder::Fs("/config.json".into())),
                        share: Some(ResourceBuilder::Fs("/share".into())),
                    }],
                    www: Some(ResourceBuilder::Fs("/www".into())),
                    share: Some(ResourceBuilder::Fs("/share".into())),
                }
            );
        }

        {
            let path = dir_path.join("manifest.json");
            let manifest_json_data = serde_json::json!({
                    "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_app_manifest.schema.json",
                    "modules": [{
                        "package": "module.hmod",
                        "name": "module_name",
                        "config": "config.json",
                        "share": "share"
                    }],
                    "www": "www",
                    "share": "share"
                }).to_string();
            std::fs::write(&path, manifest_json_data).unwrap();
            let manifest = Manifest::from_file(&path).unwrap();
            assert_eq!(
                manifest,
                Manifest {
                    name: "app".to_string(),
                    icon: ResourceBuilder::Fs(dir_path.join("icon.svg")),
                    metadata: ResourceBuilder::Fs(dir_path.join("metadata.json")),
                    modules: vec![ManifestModule {
                        package: ResourceBuilder::Fs(dir_path.join("module.hmod")),
                        name: Some("module_name".to_string()),
                        config: Some(ResourceBuilder::Fs(dir_path.join("config.json"))),
                        share: Some(ResourceBuilder::Fs(dir_path.join("share"))),
                    }],
                    www: Some(ResourceBuilder::Fs(dir_path.join("www"))),
                    share: Some(ResourceBuilder::Fs(dir_path.join("share"))),
                }
            );
        }

        {
            let path = dir_path.join("manifest.json");
            let manifest_json_data = serde_json::json!({
                    "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_app_manifest.schema.json",
                }).to_string();
            std::fs::write(&path, manifest_json_data).unwrap();
            assert!(Manifest::from_file(&path).is_err());
        }
    }
}
