//! WASM module package manifet.json struct.

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Manifest file open and read error.
#[derive(thiserror::Error, Debug)]
#[error("Cannot open and read WASM module package manifest file at {0}")]
pub(crate) struct ManifestFileError(PathBuf);

/// WASM module package manifest reading error.
#[derive(thiserror::Error, Debug)]
#[error("WASM module manifest json file reading error: {0}")]
pub(crate) struct ManifestReadingError(String);

/// WASM module package manifet.json definition.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Manifest {
    /// Path to the metadata JSON file.
    #[serde(default = "Manifest::default_metadata_path")]
    metadata: PathBuf,
    /// Path to the  WASM component file.
    #[serde(default = "Manifest::default_component_path")]
    component: PathBuf,
    /// Path to the config JSON file.
    config: Option<PathBuf>,
    /// Path to the config schema JSON file.
    config_schema: Option<PathBuf>,
    /// Path to the settings schema JSON file.
    settings_schema: Option<PathBuf>,
    /// Path to the share directory.
    share: Option<PathBuf>,
}

impl Manifest {
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
        let file = std::fs::File::open(&path_to_manifest)
            .map_err(|_| ManifestFileError(path_to_manifest.as_ref().into()))?;
        Ok(serde_json::from_reader(file).map_err(|err| ManifestReadingError(err.to_string()))?)
    }
}

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;

    #[test]
    fn manifest_json_deserialize_test() {
        let value = serde_json::json!({
            "metadata": "metadata.json",
            "component": "module.wasm",
            "config": "config.json",
            "config_schema": "config.schema.json",
            "settings_schema": "settings.schema.json",
            "share": "share"
        });
        let manifest = serde_json::from_value::<Manifest>(value).expect("failed to deserialize");
        let expected = Manifest {
            metadata: PathBuf::from("metadata.json"),
            component: PathBuf::from("module.wasm"),
            config: Some(PathBuf::from("config.json")),
            config_schema: Some(PathBuf::from("config.schema.json")),
            settings_schema: Some(PathBuf::from("settings.schema.json")),
            share: Some(PathBuf::from("share")),
        };
        assert_eq!(manifest, expected);

        let value = serde_json::json!({
            "metadata": "metadata.json",
            "component": "module.wasm",
        });
        let manifest = serde_json::from_value::<Manifest>(value).expect("failed to deserialize");
        let expected = Manifest {
            metadata: PathBuf::from("metadata.json"),
            component: PathBuf::from("module.wasm"),
            config: None,
            config_schema: None,
            settings_schema: None,
            share: None,
        };
        assert_eq!(manifest, expected);

        let value = serde_json::json!({});
        let manifest = serde_json::from_value::<Manifest>(value).expect("failed to deserialize");
        let expected = Manifest {
            metadata: PathBuf::from("metadata.json"),
            component: PathBuf::from("module.wasm"),
            config: None,
            config_schema: None,
            settings_schema: None,
            share: None,
        };
        assert_eq!(manifest, expected);
    }

    #[test]
    fn manifest_from_path_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let path = dir.path().join("manifest.json");
        let manifest_json_data = serde_json::json!({}).to_string();
        std::fs::write(&path, manifest_json_data).expect("Cannot create manifest.json file");

        let manifest = Manifest::from_file(path).expect("Cannot create manifest");
        assert_eq!(manifest, Manifest {
            metadata: PathBuf::from("metadata.json"),
            component: PathBuf::from("module.wasm"),
            config: None,
            config_schema: None,
            settings_schema: None,
            share: None,
        });
    }
}
