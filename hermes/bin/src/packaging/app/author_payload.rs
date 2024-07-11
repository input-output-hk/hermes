//! A signature payload object for author.cose Hermes application package.

use crate::packaging::{
    hash::Blake2b256, schema_validation::SchemaValidator, sign::signature::SignaturePayloadEncoding,
};

/// A signature payload object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SignaturePayload {
    /// Hash of the metadata JSON file.
    metadata: Blake2b256,
    /// Hash of the WASM component file.
    component: Blake2b256,
    /// Config instance.
    config: Option<SignaturePayloadConfig>,
    /// Settings instance.
    settings: Option<SignaturePayloadSettings>,
    /// Hash of the share directory content.
    share: Option<Blake2b256>,
}

/// A `SignaturePayload` config object.
#[derive(Clone, Debug, PartialEq, Eq)]
struct SignaturePayloadConfig {
    /// Hash of the config JSON file.
    file: Option<Blake2b256>,
    /// Hash of the config schema JSON file.
    schema: Blake2b256,
}

/// A `SignaturePayload` settings object.
#[derive(Clone, Debug, PartialEq, Eq)]
struct SignaturePayloadSettings {
    /// Hash of the settings schema JSON file.
    schema: Blake2b256,
}

/// `SignaturePayload` builder object.
pub(crate) struct SignaturePayloadBuilder {
    /// Hash of the metadata JSON file.
    metadata: Blake2b256,
    /// Hash of the WASM component file.
    component: Blake2b256,
    /// Hash of the config JSON file.
    config_file: Option<Blake2b256>,
    /// Hash of the config schema JSON file.
    config_schema: Option<Blake2b256>,
    /// Hash of the settings schema JSON file.
    settings_schema: Option<Blake2b256>,
    /// Hash of the share directory content.
    share: Option<Blake2b256>,
}

impl SignaturePayloadBuilder {
    /// Create a new `SignaturePayloadBuilder`.
    pub(crate) fn new(metadata: Blake2b256, component: Blake2b256) -> Self {
        Self {
            metadata,
            component,
            config_file: None,
            config_schema: None,
            settings_schema: None,
            share: None,
        }
    }

    /// Set the config file hash.
    pub(crate) fn with_config_file(&mut self, file: Blake2b256) {
        self.config_file = Some(file);
    }

    /// Set the config schema hash.
    pub(crate) fn with_config_schema(&mut self, schema: Blake2b256) {
        self.config_schema = Some(schema);
    }

    /// Set the settings schema hash.
    pub(crate) fn with_settings_schema(&mut self, schema: Blake2b256) {
        self.settings_schema = Some(schema);
    }

    /// Set the share directory hash.
    pub(crate) fn with_share(&mut self, share: Blake2b256) {
        self.share = Some(share);
    }

    /// Create a new `SignaturePayload`.
    pub(crate) fn build(self) -> SignaturePayload {
        SignaturePayload {
            metadata: self.metadata,
            component: self.component,
            config: self.config_schema.map(|schema| {
                SignaturePayloadConfig {
                    file: self.config_file,
                    schema,
                }
            }),
            settings: self
                .settings_schema
                .map(|schema| SignaturePayloadSettings { schema }),
            share: self.share,
        }
    }
}

/// WASM module cose signature payload JSON schema.
const SIGNATURE_PAYLOAD_SCHEMA: &str =
    include_str!("../../../../schemas/hermes_module_cose_author_payload.schema.json");

impl SignaturePayloadEncoding for SignaturePayload {
    fn to_json(&self) -> serde_json::Value {
        let mut json = serde_json::Map::new();
        json.insert("metadata".to_string(), self.metadata.to_hex().into());
        json.insert("component".to_string(), self.component.to_hex().into());
        if let Some(config) = &self.config {
            let mut config_json = serde_json::Map::new();

            config_json.insert("schema".to_string(), config.schema.to_hex().into());
            if let Some(file) = &config.file {
                config_json.insert("file".to_string(), file.to_hex().into());
            }

            json.insert("config".to_string(), config_json.into());
        }
        if let Some(settings) = &self.settings {
            json.insert(
                "settings".to_string(),
                serde_json::json!({
                    "schema": settings.schema.to_hex()
                }),
            );
        }
        if let Some(share) = &self.share {
            json.insert("share".to_string(), share.to_hex().into());
        }

        json.into()
    }

    fn from_json(json: serde_json::Value) -> anyhow::Result<Self>
    where Self: Sized {
        let schema_validator = SchemaValidator::from_str(SIGNATURE_PAYLOAD_SCHEMA)?;
        schema_validator.validate(&json)?;

        let json = json
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Signature payload JSON is not an object"))?;

        let metadata = json
            .get("metadata")
            .ok_or_else(|| anyhow::anyhow!("Missing metadata field"))?
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid metadata field"))
            .map(Blake2b256::from_hex)??;

        let component = json
            .get("component")
            .ok_or_else(|| anyhow::anyhow!("Missing component field"))?
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid component field"))
            .map(Blake2b256::from_hex)??;

        let config = json
            .get("config")
            .and_then(|val| val.as_object())
            .map(|config| -> anyhow::Result<_> {
                let file = config
                    .get("file")
                    .map(|file| {
                        file.as_str()
                            .ok_or_else(|| anyhow::anyhow!("Invalid file field"))
                            .map(Blake2b256::from_hex)?
                    })
                    .transpose()?;
                let schema = config
                    .get("schema")
                    .ok_or_else(|| anyhow::anyhow!("Missing schema field"))?
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid schema field"))
                    .map(Blake2b256::from_hex)??;
                Ok(SignaturePayloadConfig { file, schema })
            })
            .transpose()?;

        let settings = json
            .get("settings")
            .and_then(|val| val.as_object())
            .map(|settings| -> anyhow::Result<_> {
                let schema = settings
                    .get("schema")
                    .ok_or_else(|| anyhow::anyhow!("Missing schema field"))?
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid schema field"))
                    .map(Blake2b256::from_hex)??;
                Ok(SignaturePayloadSettings { schema })
            })
            .transpose()?;

        let share = json
            .get("share")
            .and_then(|val| val.as_str())
            .map(Blake2b256::from_hex)
            .transpose()?;

        Ok(SignaturePayload {
            metadata,
            component,
            config,
            settings,
            share,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_payload_encoding_test() {
        let hash = Blake2b256::hash(b"test");
        let schema_validator =
            SchemaValidator::from_str(SIGNATURE_PAYLOAD_SCHEMA).expect("Invalid schema");

        {
            let payload_builder = SignaturePayloadBuilder::new(hash.clone(), hash.clone());
            let payload = payload_builder.build();

            let json = payload.to_json();
            schema_validator.validate(&json).expect("Invalid JSON");

            let expected_json = serde_json::json!({
                "metadata": hash.to_hex(),
                "component": hash.to_hex(),
            });
            assert_eq!(json, expected_json);

            let payload = SignaturePayload::from_json(json).expect("Cannot parse JSON");
            assert_eq!(payload, payload);
        }

        {
            let mut payload_builder = SignaturePayloadBuilder::new(hash.clone(), hash.clone());
            payload_builder.with_config_file(hash.clone());
            payload_builder.with_config_schema(hash.clone());
            payload_builder.with_settings_schema(hash.clone());
            payload_builder.with_share(hash.clone());
            let payload = payload_builder.build();

            let json = payload.to_json();
            schema_validator.validate(&json).expect("Invalid JSON");

            let expected_json = serde_json::json!({
                "metadata": hash.to_hex(),
                "component": hash.to_hex(),
                "config": {
                    "file": hash.to_hex(),
                    "schema": hash.to_hex(),
                },
                "settings": {
                    "schema": hash.to_hex(),
                },
                "share": hash.to_hex(),
            });
            assert_eq!(json, expected_json);

            let payload = SignaturePayload::from_json(json).expect(
                "Cannot parse
            JSON",
            );
            assert_eq!(payload, payload);
        }
    }
}
