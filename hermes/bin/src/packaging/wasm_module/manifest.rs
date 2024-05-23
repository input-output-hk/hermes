//! WASM module package manifest.json struct.

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

/// WASM module package manifest.json definition.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Manifest {
    /// Path to the metadata JSON file.
    #[serde(default = "Manifest::default_metadata_path")]
    pub(crate) metadata: PathBuf,
    /// Path to the  WASM component file.
    #[serde(default = "Manifest::default_component_path")]
    pub(crate) component: PathBuf,
    /// Path to the config JSON file.
    pub(crate) config: Option<PathBuf>,
    /// Path to the config schema JSON file.
    pub(crate) config_schema: Option<PathBuf>,
    /// Path to the settings schema JSON file.
    pub(crate) settings_schema: Option<PathBuf>,
    /// Path to the share directory.
    pub(crate) share: Option<PathBuf>,
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
        let path = path_to_manifest.as_ref();
        let file = std::fs::File::open(path).map_err(|_| ManifestFileError(path.into()))?;

        let dir_path = path
            .parent()
            .ok_or_else(|| ManifestFileError(path.into()))?;

        let mut manifest: Manifest =
            serde_json::from_reader(file).map_err(|err| ManifestReadingError(err.to_string()))?;

        if manifest.metadata.is_relative() {
            manifest.metadata = dir_path.join(&manifest.metadata);
        }
        if manifest.component.is_relative() {
            manifest.component = dir_path.join(&manifest.component);
        }
        if let Some(config) = &mut manifest.config {
            if config.is_relative() {
                *config = dir_path.join(&config);
            }
        }
        if let Some(config_schema) = &mut manifest.config_schema {
            if config_schema.is_relative() {
                *config_schema = dir_path.join(&config_schema);
            }
        }
        if let Some(settings_schema) = &mut manifest.settings_schema {
            if settings_schema.is_relative() {
                *settings_schema = dir_path.join(&settings_schema);
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
    fn manifest_from_file_test() {
        let dir = TempDir::new().expect("cannot create temp dir");
        let dir_path = dir.path();

        let path = dir_path.join("manifest.json");
        let manifest_json_data = serde_json::json!({
            "metadata": "metadata.json",
            "component": "module.wasm",
            "config": "config.json",
            "config_schema": "config.schema.json",
            "settings_schema": "settings.schema.json",
            "share": "share"
        })
        .to_string();
        std::fs::write(&path, manifest_json_data).expect("Cannot create manifest.json file");
        let manifest = Manifest::from_file(&path).expect("Cannot create manifest");
        assert_eq!(manifest, Manifest {
            metadata: dir_path.join("metadata.json"),
            component: dir_path.join("module.wasm"),
            config: Some(dir_path.join("config.json")),
            config_schema: Some(dir_path.join("config.schema.json")),
            settings_schema: Some(dir_path.join("settings.schema.json")),
            share: Some(dir_path.join("share")),
        });

        let manifest_json_data = serde_json::json!({
            "metadata": "/metadata.json",
            "component": "/module.wasm",
            "config": "/config.json",
            "config_schema": "/config.schema.json",
            "settings_schema": "/settings.schema.json",
            "share": "/share"
        })
        .to_string();
        std::fs::write(&path, manifest_json_data).expect("Cannot create manifest.json file");
        let manifest = Manifest::from_file(path).expect("Cannot create manifest");
        assert_eq!(manifest, Manifest {
            metadata: PathBuf::from("/metadata.json"),
            component: PathBuf::from("/module.wasm"),
            config: Some(PathBuf::from("/config.json")),
            config_schema: Some(PathBuf::from("/config.schema.json")),
            settings_schema: Some(PathBuf::from("/settings.schema.json")),
            share: Some(PathBuf::from("/share")),
        });

        let path = dir_path.join("manifest.json");
        let manifest_json_data = serde_json::json!({}).to_string();
        std::fs::write(&path, manifest_json_data).expect("Cannot create manifest.json file");
        let manifest = Manifest::from_file(&path).expect("Cannot create manifest");
        assert_eq!(manifest, Manifest {
            metadata: dir_path.join("metadata.json"),
            component: dir_path.join("module.wasm"),
            config: None,
            config_schema: None,
            settings_schema: None,
            share: None,
        });
    }
}
