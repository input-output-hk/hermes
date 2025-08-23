//! WASM module package manifest.json struct.

use std::path::Path;

use super::super::{schema_validation::SchemaValidator, FileError};
use crate::hdf5::resources::ResourceBuilder;

/// WASM module package manifest.json definition.
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(PartialEq, Eq)]
pub(crate) struct Manifest {
    /// Package name.
    pub(crate) name: String,
    /// Path to the metadata JSON file.
    pub(crate) metadata: ResourceBuilder,
    /// Path to the  WASM component file.
    pub(crate) component: ResourceBuilder,
    /// WASM module config.
    pub(crate) config: Option<ManifestConfig>,
    /// WASM module settings.
    pub(crate) settings: Option<ManifestSettings>,
    /// Path to the share directory.
    pub(crate) share: Option<ResourceBuilder>,
}

/// `Manifest` config definition.
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(PartialEq, Eq)]
pub(crate) struct ManifestConfig {
    /// Path to the config JSON file.
    pub(crate) file: Option<ResourceBuilder>,
    /// Path to the config schema JSON file.
    pub(crate) schema: ResourceBuilder,
}

/// `Manifest` settings definition.
#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(PartialEq, Eq)]
pub(crate) struct ManifestSettings {
    /// Path to the settings schema JSON file.
    pub(crate) schema: ResourceBuilder,
}

impl Manifest {
    /// WASM module manifest JSON schema.
    const MANIFEST_SCHEMA: &'static str =
        include_str!("../../../../schemas/hermes_module_manifest.schema.json");

    /// Default package name.
    fn default_package_name() -> String {
        String::from("module")
    }

    /// Default metadata JSON file path.
    fn default_metadata_path() -> ResourceBuilder {
        ResourceBuilder::Fs("metadata.json".into())
    }

    /// Default WASM component file path.
    fn default_component_path() -> ResourceBuilder {
        ResourceBuilder::Fs("module.wasm".into())
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
        manifest.component.make_relative_to(dir_path);
        if let Some(config) = manifest.config.as_mut() {
            if let Some(config_file) = config.file.as_mut() {
                config_file.make_relative_to(dir_path);
            }
            config.schema.make_relative_to(dir_path);
        }
        if let Some(settings) = manifest.settings.as_mut() {
            settings.schema.make_relative_to(dir_path);
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
        #[serde(default = "super::Manifest::default_metadata_path")]
        metadata: ResourceBuilder,
        #[serde(default = "super::Manifest::default_component_path")]
        component: ResourceBuilder,
        config: Option<ConfigSerde>,
        settings: Option<SettingsSerde>,
        share: Option<ResourceBuilder>,
    }

    #[derive(Deserialize)]
    struct ConfigSerde {
        file: Option<ResourceBuilder>,
        schema: ResourceBuilder,
    }

    #[derive(Deserialize)]
    struct SettingsSerde {
        schema: ResourceBuilder,
    }

    impl From<ManifestSerde> for super::Manifest {
        fn from(def: ManifestSerde) -> Self {
            Self {
                name: def.name,
                metadata: def.metadata,
                component: def.component,
                config: def.config.map(|def| {
                    super::ManifestConfig {
                        file: def.file,
                        schema: def.schema,
                    }
                }),
                settings: def
                    .settings
                    .map(|def| super::ManifestSettings { schema: def.schema }),
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
    fn manifest_from_file_test() {
        let dir = TempDir::new().unwrap();
        let dir_path = dir.path();

        {
            let path = dir_path.join("manifest.json");
            let manifest_json_data = serde_json::json!({
                    "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_module_manifest.schema.json",
                    "name": "module_name",
                    "metadata": "metadata.json",
                    "component": "module.wasm",
                    "config": {
                        "file": "config.json",
                        "schema": "config.schema.json"
                    },
                    "settings": {
                        "schema": "settings.schema.json"
                    },
                    "share": "share"
                }).to_string();
            std::fs::write(&path, manifest_json_data).unwrap();
            let manifest = Manifest::from_file(&path).unwrap();
            assert_eq!(manifest, Manifest {
                name: "module_name".to_string(),
                metadata: ResourceBuilder::Fs(dir_path.join("metadata.json")),
                component: ResourceBuilder::Fs(dir_path.join("module.wasm")),
                config: ManifestConfig {
                    file: Some(ResourceBuilder::Fs(dir_path.join("config.json"))),
                    schema: ResourceBuilder::Fs(dir_path.join("config.schema.json")),
                }
                .into(),
                settings: ManifestSettings {
                    schema: ResourceBuilder::Fs(dir_path.join("settings.schema.json")),
                }
                .into(),
                share: Some(ResourceBuilder::Fs(dir_path.join("share"))),
            });
        }

        {
            let path = dir_path.join("manifest.json");
            let manifest_json_data = serde_json::json!({
                    "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_module_manifest.schema.json",
                    "metadata": "/metadata.json",
                    "component": "/module.wasm",
                    "config": {
                        "file": "/config.json",
                        "schema": "/config.schema.json"
                    },
                    "settings": {
                        "schema": "/settings.schema.json"
                    },
                    "share": "/share"
                }).to_string();
            std::fs::write(&path, manifest_json_data).unwrap();
            let manifest = Manifest::from_file(path).unwrap();
            assert_eq!(manifest, Manifest {
                name: "module".to_string(),
                metadata: ResourceBuilder::Fs("/metadata.json".into()),
                component: ResourceBuilder::Fs("/module.wasm".into()),
                config: ManifestConfig {
                    file: Some(ResourceBuilder::Fs("/config.json".into())),
                    schema: ResourceBuilder::Fs("/config.schema.json".into()),
                }
                .into(),
                settings: ManifestSettings {
                    schema: ResourceBuilder::Fs("/settings.schema.json".into()),
                }
                .into(),
                share: Some(ResourceBuilder::Fs("/share".into())),
            });
        }

        {
            let path = dir_path.join("manifest.json");
            let manifest_json_data = serde_json::json!({
                "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_module_manifest.schema.json",
                }).to_string();
            std::fs::write(&path, manifest_json_data).unwrap();
            let manifest = Manifest::from_file(&path).unwrap();
            assert_eq!(manifest, Manifest {
                name: "module".to_string(),
                metadata: ResourceBuilder::Fs(dir_path.join("metadata.json")),
                component: ResourceBuilder::Fs(dir_path.join("module.wasm")),
                config: None,
                settings: None,
                share: None,
            });
        }
    }
}
