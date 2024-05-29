//! WASM module package manifest.json struct.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::packaging::schema_validation::SchemaValidator;

/// Manifest file open and read error.
#[derive(thiserror::Error, Debug)]
#[error("Cannot open and read WASM module package manifest file at {0}")]
pub(crate) struct ManifestFileError(PathBuf);

/// WASM module package manifest reading error.
#[derive(thiserror::Error, Debug)]
#[error("WASM module manifest json file reading errors:\n{0}")]
pub(crate) struct ManifestReadingError(String);

/// WASM module package manifest.json definition.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Manifest {
    /// Package name.
    #[serde(default = "Manifest::default_package_name")]
    pub(crate) name: String,
    /// Path to the metadata JSON file.
    #[serde(default = "Manifest::default_metadata_path")]
    pub(crate) metadata: PathBuf,
    /// Path to the  WASM component file.
    #[serde(default = "Manifest::default_component_path")]
    pub(crate) component: PathBuf,
    /// WASM module config.
    pub(crate) config: Option<Config>,
    /// WASM module settings.
    pub(crate) settings: Option<Settings>,
    /// Path to the share directory.
    pub(crate) share: Option<PathBuf>,
}

/// WASM module config definition.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Config {
    /// Path to the config JSON file.
    pub(crate) file: Option<PathBuf>,
    /// Path to the config schema JSON file.
    pub(crate) schema: PathBuf,
}

/// WASM module settings definition.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Settings {
    /// Path to the settings schema JSON file.
    pub(crate) schema: PathBuf,
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
    fn default_metadata_path() -> PathBuf {
        PathBuf::from("metadata.json")
    }

    /// Default WASM component file path.
    fn default_component_path() -> PathBuf {
        PathBuf::from("module.wasm")
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

        if manifest.metadata.is_relative() {
            manifest.metadata = dir_path.join(&manifest.metadata);
        }
        if manifest.component.is_relative() {
            manifest.component = dir_path.join(&manifest.component);
        }

        if let Some(config) = &mut manifest.config {
            if let Some(config_file) = &mut config.file {
                if config_file.is_relative() {
                    *config_file = dir_path.join(&config_file);
                }
            }
            if config.schema.is_relative() {
                config.schema = dir_path.join(&config.schema);
            }
        }
        if let Some(settings) = &mut manifest.settings {
            if settings.schema.is_relative() {
                settings.schema = dir_path.join(&settings.schema);
            }
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
            metadata: dir_path.join("metadata.json"),
            component: dir_path.join("module.wasm"),
            config: Config {
                file: dir_path.join("config.json").into(),
                schema: dir_path.join("config.schema.json"),
            }
            .into(),
            settings: Settings {
                schema: dir_path.join("settings.schema.json"),
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
            metadata: PathBuf::from("/metadata.json"),
            component: PathBuf::from("/module.wasm"),
            config: Config {
                file: PathBuf::from("/config.json").into(),
                schema: PathBuf::from("/config.schema.json"),
            }
            .into(),
            settings: Settings {
                schema: PathBuf::from("/settings.schema.json"),
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
            metadata: dir_path.join("metadata.json"),
            component: dir_path.join("module.wasm"),
            config: None,
            settings: None,
            share: None,
        });
    }
}
