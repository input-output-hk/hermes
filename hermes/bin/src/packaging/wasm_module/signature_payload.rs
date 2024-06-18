//! A signature payload object for WASM module package.
//! Defined at `https://input-output-hk.github.io/hermes/architecture/08_concepts/hermes_packaging_requirements/wasm_modules/#wasm-component-module-signatures`.

use crate::sign::hash::Blake2b256;

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
    pub(crate) fn with_config_file(&mut self, file: Blake2b256) -> &mut Self {
        self.config_file = Some(file);
        self
    }

    /// Set the config schema hash.
    pub(crate) fn with_config_schema(&mut self, schema: Blake2b256) -> &mut Self {
        self.config_schema = Some(schema);
        self
    }

    /// Set the settings schema hash.
    pub(crate) fn with_settings_schema(&mut self, schema: Blake2b256) -> &mut Self {
        self.settings_schema = Some(schema);
        self
    }

    /// Set the share directory hash.
    pub(crate) fn with_share(&mut self, share: Blake2b256) -> &mut Self {
        self.share = Some(share);
        self
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

mod serde_def {
    //! Serde definition of the signature payload objects.
}
