//! WASM module package manifet.json struct.

use std::path::PathBuf;

use serde::Deserialize;

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
}

#[cfg(test)]
mod tests {
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
}
