//! WASM module package manifest.json struct.

use std::path::Path;

use crate::packaging::{
    resources::{fs::FsResource, Resource},
    schema_validation::SchemaValidator,
    FileError,
};

/// WASM module package manifest.json definition.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Manifest {
    /// Package name.
    pub(crate) name: String,
    /// Path to the metadata JSON file.
    pub(crate) metadata: Resource,
    /// Path to the  WASM component file.
    pub(crate) component: Resource,
    /// WASM module config.
    pub(crate) config: Option<ManifestConfig>,
    /// WASM module settings.
    pub(crate) settings: Option<ManifestSettings>,
    /// Path to the share directory.
    pub(crate) share: Option<Resource>,
}

/// `Manifest` config definition.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ManifestConfig {
    /// Path to the config JSON file.
    pub(crate) file: Option<Resource>,
    /// Path to the config schema JSON file.
    pub(crate) schema: Resource,
}

/// `Manifest` settings definition.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ManifestSettings {
    /// Path to the settings schema JSON file.
    pub(crate) schema: Resource,
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
    fn default_metadata_path() -> Resource {
        Resource::Fs(FsResource::new("metadata.json"))
    }

    /// Default WASM component file path.
    fn default_component_path() -> Resource {
        Resource::Fs(FsResource::new("module.wasm"))
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

    use crate::packaging::resources::Resource;

    #[derive(Deserialize)]
    pub(crate) struct ManifestSerde {
        #[serde(default = "super::Manifest::default_package_name")]
        name: String,
        #[serde(default = "super::Manifest::default_metadata_path")]
        metadata: Resource,
        #[serde(default = "super::Manifest::default_component_path")]
        component: Resource,
        config: Option<ConfigSerde>,
        settings: Option<SettingsSerde>,
        share: Option<Resource>,
    }

    #[derive(Deserialize)]
    struct ConfigSerde {
        file: Option<Resource>,
        schema: Resource,
    }

    #[derive(Deserialize)]
    struct SettingsSerde {
        schema: Resource,
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

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;

    #[test]
    fn manifest_from_file_test() {
        let dir = TempDir::new().expect("cannot create temp dir");
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
            std::fs::write(&path, manifest_json_data).expect("Cannot create manifest.json file");
            let manifest = Manifest::from_file(&path).expect("Cannot create manifest");
            assert_eq!(manifest, Manifest {
                name: "module_name".to_string(),
                metadata: Resource::Fs(FsResource::new(dir_path.join("metadata.json"))),
                component: Resource::Fs(FsResource::new(dir_path.join("module.wasm"))),
                config: ManifestConfig {
                    file: Some(Resource::Fs(FsResource::new(dir_path.join("config.json")))),
                    schema: Resource::Fs(FsResource::new(dir_path.join("config.schema.json"))),
                }
                .into(),
                settings: ManifestSettings {
                    schema: Resource::Fs(FsResource::new(dir_path.join("settings.schema.json"))),
                }
                .into(),
                share: Some(Resource::Fs(FsResource::new(dir_path.join("share")))),
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
            std::fs::write(&path, manifest_json_data).expect("Cannot create manifest.json file");
            let manifest = Manifest::from_file(path).expect("Cannot create manifest");
            assert_eq!(manifest, Manifest {
                name: "module".to_string(),
                metadata: Resource::Fs(FsResource::new("/metadata.json")),
                component: Resource::Fs(FsResource::new("/module.wasm")),
                config: ManifestConfig {
                    file: Some(Resource::Fs(FsResource::new("/config.json"))),
                    schema: Resource::Fs(FsResource::new("/config.schema.json")),
                }
                .into(),
                settings: ManifestSettings {
                    schema: Resource::Fs(FsResource::new("/settings.schema.json")),
                }
                .into(),
                share: Some(Resource::Fs(FsResource::new("/share"))),
            });
        }

        {
            let path = dir_path.join("manifest.json");
            let manifest_json_data = serde_json::json!({
                "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_module_manifest.schema.json",
                }).to_string();
            std::fs::write(&path, manifest_json_data).expect("Cannot create manifest.json file");
            let manifest = Manifest::from_file(&path).expect("Cannot create manifest");
            assert_eq!(manifest, Manifest {
                name: "module".to_string(),
                metadata: Resource::Fs(FsResource::new(dir_path.join("metadata.json"))),
                component: Resource::Fs(FsResource::new(dir_path.join("module.wasm"))),
                config: None,
                settings: None,
                share: None,
            });
        }
    }
}
