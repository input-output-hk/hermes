//! WASM module package manifest.json struct.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::packaging::{resourses::Resource, schema_validation::SchemaValidator};

/// Manifest file open and read error.
#[derive(thiserror::Error, Debug)]
#[error("Cannot open and read WASM module package manifest file at {0}")]
pub(crate) struct ManifestFileError(PathBuf);

/// WASM module package manifest reading error.
#[derive(thiserror::Error, Debug)]
#[error("WASM module manifest json file reading errors:\n{0}")]
pub(crate) struct ManifestReadingError(String);

/// WASM module package manifest.json definition.
#[derive(Deserialize, Debug, PartialEq, Eq)]
pub(crate) struct Manifest {
    /// Package name.
    #[serde(default = "Manifest::default_package_name")]
    pub(crate) name: String,
    /// Path to the metadata JSON file.
    #[serde(default = "Manifest::default_metadata_path")]
    pub(crate) metadata: Resource,
    /// Path to the  WASM component file.
    #[serde(default = "Manifest::default_component_path")]
    pub(crate) component: Resource,
    /// WASM module config.
    pub(crate) config: Option<Config>,
    /// WASM module settings.
    pub(crate) settings: Option<Settings>,
    /// Path to the share directory.
    pub(crate) share: Option<PathBuf>,
}

/// WASM module config definition.
#[derive(Deserialize, Debug, PartialEq, Eq)]
pub(crate) struct Config {
    /// Path to the config JSON file.
    pub(crate) file: Option<Resource>,
    /// Path to the config schema JSON file.
    pub(crate) schema: Resource,
}

/// WASM module settings definition.
#[derive(Deserialize, Debug, PartialEq, Eq)]
pub(crate) struct Settings {
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
        PathBuf::from("metadata.json").into()
    }

    /// Default WASM component file path.
    fn default_component_path() -> Resource {
        PathBuf::from("module.wasm").into()
    }

    /// Create a Manifest from a path.
    pub(crate) fn from_file<P: AsRef<Path>>(path_to_manifest: P) -> anyhow::Result<Self> {
        let path = path_to_manifest.as_ref();
        let file = std::fs::File::open(path).map_err(|_| ManifestFileError(path.into()))?;

        let dir_path = path
            .parent()
            .ok_or_else(|| ManifestFileError(path.into()))?;

        let schema_validator = SchemaValidator::from_str(Self::MANIFEST_SCHEMA)?;
        let mut manifest: Manifest = schema_validator
            .deserialize_and_validate(file)
            .map_err(|err| ManifestReadingError(err.to_string()))?;

        manifest.metadata.make_relative_to(dir_path);
        manifest.component.make_relative_to(dir_path);
        if let Some(config) = &mut manifest.config {
            if let Some(config_file) = &mut config.file {
                config_file.make_relative_to(dir_path);
            }
            config.schema.make_relative_to(dir_path);
        }
        if let Some(settings) = &mut manifest.settings {
            settings.schema.make_relative_to(dir_path);
        }
        if let Some(share) = &mut manifest.share {
            if share.is_relative() {
                *share = dir_path.join(&share);
            }
        }

        Ok(manifest)
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

        let path = dir_path.join("manifest.json");
        let manifest_json_data = serde_json::json!({
            "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_module_manifest.schema.json",
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
        })
        .to_string();
        std::fs::write(&path, manifest_json_data).expect("Cannot create manifest.json file");
        let manifest = Manifest::from_file(&path).expect("Cannot create manifest");
        assert_eq!(manifest, Manifest {
            name: "module".to_string(),
            metadata: dir_path.join("metadata.json").into(),
            component: dir_path.join("module.wasm").into(),
            config: Config {
                file: Some(dir_path.join("config.json").into()),
                schema: dir_path.join("config.schema.json").into(),
            }
            .into(),
            settings: Settings {
                schema: dir_path.join("settings.schema.json").into(),
            }
            .into(),
            share: dir_path.join("share").into(),
        });

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
        })
        .to_string();
        std::fs::write(&path, manifest_json_data).expect("Cannot create manifest.json file");
        let manifest = Manifest::from_file(path).expect("Cannot create manifest");
        assert_eq!(manifest, Manifest {
            name: "module".to_string(),
            metadata: PathBuf::from("/metadata.json").into(),
            component: PathBuf::from("/module.wasm").into(),
            config: Config {
                file: Some(PathBuf::from("/config.json").into()),
                schema: PathBuf::from("/config.schema.json").into(),
            }
            .into(),
            settings: Settings {
                schema: PathBuf::from("/settings.schema.json").into(),
            }
            .into(),
            share: PathBuf::from("/share").into(),
        });

        let path = dir_path.join("manifest.json");
        let manifest_json_data = serde_json::json!({
            "$schema": "https://raw.githubusercontent.com/input-output-hk/hermes/main/hermes/schemas/hermes_module_manifest.schema.json",
        })
        .to_string();
        std::fs::write(&path, manifest_json_data).expect("Cannot create manifest.json file");
        let manifest = Manifest::from_file(&path).expect("Cannot create manifest");
        assert_eq!(manifest, Manifest {
            name: "module".to_string(),
            metadata: dir_path.join("metadata.json").into(),
            component: dir_path.join("module.wasm").into(),
            config: None,
            settings: None,
            share: None,
        });
    }
}
