//! A signature payload object for author.cose Hermes application package.
//! Defined at `https://input-output-hk.github.io/hermes/architecture/08_concepts/hermes_packaging_requirements/app_signatures/#wasm-component-module-signatures`.

use crate::packaging::{
    hash::Blake2b256, schema_validation::SchemaValidator, sign::signature::SignaturePayloadEncoding,
};

/// A signature payload object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SignaturePayload {
    /// Hash of the metadata JSON file.
    metadata: Blake2b256,
    /// Hash of the icon SVG file.
    icon: Blake2b256,
    /// Modules list.
    modules: Vec<SignaturePayloadModule>,
    /// Hash of the www directory content.
    www: Option<Blake2b256>,
    /// Hash of the share directory content.
    share: Option<Blake2b256>,
}

/// A `SignaturePayload` module object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SignaturePayloadModule {
    /// Name of the WASM module.
    name: String,
    /// Hash of the of the entire WASM module package.
    package: Blake2b256,
    /// Hash of the replaced module's config.json package file.
    config: Option<Blake2b256>,
    /// Hash of the whole replaced module's share package directory.
    share: Option<Blake2b256>,
}

/// `SignaturePayload` builder object.
pub(crate) struct SignaturePayloadBuilder {
    /// Hash of the metadata JSON file.
    metadata: Blake2b256,
    /// Hash of the icon SVG file.
    icon: Blake2b256,
    /// Modules list.
    modules: Vec<SignaturePayloadModule>,
    /// Hash of the www directory content.
    www: Option<Blake2b256>,
    /// Hash of the share directory content.
    share: Option<Blake2b256>,
}

impl SignaturePayloadBuilder {
    /// Create a new `SignaturePayloadBuilder`.
    pub(crate) fn new(metadata: Blake2b256, icon: Blake2b256) -> Self {
        Self {
            metadata,
            icon,
            modules: vec![],
            www: None,
            share: None,
        }
    }

    /// Add a new `SignaturePayloadModule` into the list.
    pub(crate) fn with_module(&mut self, module: SignaturePayloadModule) {
        self.modules.push(module);
    }

    /// Set the www directory hash.
    pub(crate) fn with_www(&mut self, www: Blake2b256) {
        self.www = Some(www);
    }

    /// Set the share directory hash.
    pub(crate) fn with_share(&mut self, share: Blake2b256) {
        self.share = Some(share);
    }

    /// Create a new `SignaturePayload`.
    pub(crate) fn build(self) -> SignaturePayload {
        SignaturePayload {
            metadata: self.metadata,
            icon: self.icon,
            modules: self.modules,
            www: self.www,
            share: self.share,
        }
    }
}

/// `SignaturePayload` builder object.
pub(crate) struct SignaturePayloadModuleBuilder {
    /// Name of the WASM module.
    name: String,
    /// Hash of the of the entire WASM module package.
    package: Blake2b256,
    /// Hash of the replaced module's config.json package file.
    config: Option<Blake2b256>,
    /// Hash of the whole replaced module's share package directory.
    share: Option<Blake2b256>,
}

impl SignaturePayloadModuleBuilder {
    /// Create a new `SignaturePayloadModuleBuilder`.
    pub(crate) fn new(name: String, package: Blake2b256) -> Self {
        Self {
            name,
            package,
            config: None,
            share: None,
        }
    }

    /// Set the config.json file hash.
    pub(crate) fn with_config(&mut self, config: Blake2b256) {
        self.config = Some(config);
    }

    /// Set the share directory hash.
    pub(crate) fn with_share(&mut self, share: Blake2b256) {
        self.share = Some(share);
    }

    /// Create a new `SignaturePayloadModule`.
    pub(crate) fn build(self) -> SignaturePayloadModule {
        SignaturePayloadModule {
            name: self.name,
            package: self.package,
            config: self.config,
            share: self.share,
        }
    }
}

/// WASM module cose signature payload JSON schema.
const SIGNATURE_PAYLOAD_SCHEMA: &str =
    include_str!("../../../../schemas/hermes_app_cose_author_payload.schema.json");

impl SignaturePayloadEncoding for SignaturePayload {
    fn to_json(&self) -> serde_json::Value {
        let mut json = serde_json::Map::new();
        json.insert("metadata".into(), self.metadata.to_hex().into());
        json.insert("icon".into(), self.icon.to_hex().into());

        if !self.modules.is_empty() {
            let modules: Vec<serde_json::Value> = self
                .modules
                .iter()
                .map(|module| {
                    let mut json = serde_json::Map::new();
                    json.insert("name".into(), module.name.clone().into());
                    json.insert("package".into(), module.package.to_hex().into());
                    if let Some(config) = &module.config {
                        json.insert("config".into(), config.to_hex().into());
                    }
                    if let Some(share) = &module.share {
                        json.insert("share".into(), share.to_hex().into());
                    }
                    json.into()
                })
                .collect();
            json.insert("modules".into(), modules.into());
        }

        if let Some(www) = &self.www {
            json.insert("www".into(), www.to_hex().into());
        }
        if let Some(share) = &self.share {
            json.insert("share".into(), share.to_hex().into());
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

        let icon = json
            .get("icon")
            .ok_or_else(|| anyhow::anyhow!("Missing icon field"))?
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid icon field"))
            .map(Blake2b256::from_hex)??;

        let json_modules_array: &[serde_json::Value] = json
            .get("modules")
            .and_then(|val| val.as_array())
            .map_or(&[], |val| val.as_slice());

        let mut modules = Vec::new();
        for json_module in json_modules_array {
            let name = json_module
                .get("name")
                .ok_or_else(|| anyhow::anyhow!("Missing name field"))?
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid name field"))?
                .to_string();

            let package = json_module
                .get("package")
                .ok_or_else(|| anyhow::anyhow!("Missing package field"))?
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid package field"))
                .map(Blake2b256::from_hex)??;

            let config = json_module
                .get("config")
                .and_then(|val| val.as_str())
                .map(Blake2b256::from_hex)
                .transpose()?;

            let share = json_module
                .get("share")
                .and_then(|val| val.as_str())
                .map(Blake2b256::from_hex)
                .transpose()?;

            modules.push(SignaturePayloadModule {
                name,
                package,
                config,
                share,
            });
        }

        let www = json
            .get("www")
            .and_then(|val| val.as_str())
            .map(Blake2b256::from_hex)
            .transpose()?;

        let share = json
            .get("share")
            .and_then(|val| val.as_str())
            .map(Blake2b256::from_hex)
            .transpose()?;

        Ok(SignaturePayload {
            metadata,
            icon,
            modules,
            www,
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
                "icon": hash.to_hex(),
            });
            assert_eq!(json, expected_json);

            let payload = SignaturePayload::from_json(json).expect("Cannot parse JSON");
            assert_eq!(payload, payload);
        }

        {
            let payload_module_builder =
                SignaturePayloadModuleBuilder::new("module_1".to_string(), hash.clone());

            let mut payload_builder = SignaturePayloadBuilder::new(hash.clone(), hash.clone());
            payload_builder.with_www(hash.clone());
            payload_builder.with_share(hash.clone());
            payload_builder.with_module(payload_module_builder.build());
            let payload = payload_builder.build();

            let json = payload.to_json();
            schema_validator.validate(&json).expect("Invalid JSON");

            let expected_json = serde_json::json!({
                "metadata": hash.to_hex(),
                "icon": hash.to_hex(),
                "modules": [
                    {
                        "name": "module_1",
                        "package": hash.to_hex(),
                    }
                ],
                "www": hash.to_hex(),
                "share": hash.to_hex(),
            });
            assert_eq!(json, expected_json);

            let payload = SignaturePayload::from_json(json).expect("Cannot parse JSON");
            assert_eq!(payload, payload);
        }

        {
            let mut payload_module_builder =
                SignaturePayloadModuleBuilder::new("module_1".to_string(), hash.clone());
            payload_module_builder.with_config(hash.clone());
            payload_module_builder.with_share(hash.clone());

            let mut payload_builder = SignaturePayloadBuilder::new(hash.clone(), hash.clone());
            payload_builder.with_www(hash.clone());
            payload_builder.with_share(hash.clone());
            payload_builder.with_module(payload_module_builder.build());
            let payload = payload_builder.build();

            let json = payload.to_json();
            schema_validator.validate(&json).expect("Invalid JSON");

            let expected_json = serde_json::json!({
                "metadata": hash.to_hex(),
                "icon": hash.to_hex(),
                "modules": [
                    {
                        "name": "module_1",
                        "package": hash.to_hex(),
                        "config": hash.to_hex(),
                        "share": hash.to_hex(),
                    }
                ],
                "www": hash.to_hex(),
                "share": hash.to_hex(),
            });
            assert_eq!(json, expected_json);

            let payload = SignaturePayload::from_json(json).expect("Cannot parse JSON");
            assert_eq!(payload, payload);
        }
    }
}
