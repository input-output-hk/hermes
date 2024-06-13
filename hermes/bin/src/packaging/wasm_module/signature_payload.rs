//! A signature payload object for WASM module package.
//! Defined at `https://input-output-hk.github.io/hermes/architecture/08_concepts/hermes_packaging_requirements/wasm_modules/#wasm-component-module-signatures`.

use crate::sign::hash::Hash;

/// A signature payload object.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct SignaturePayload {
    /// Hash of the metadata JSON file.
    pub(crate) metadata: Hash,
    /// Hash of the WASM component file.
    pub(crate) component: Hash,
    /// Signature payload config instance.
    pub(crate) config: Option<Config>,
    /// Signature payload settings instance.
    pub(crate) settings: Option<Settings>,
    /// Hash of the share directory content.
    pub(crate) share: Option<Hash>,
}

/// A signature payload config object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Config {
    /// Hash of the config JSON file.
    pub(crate) file: Option<Hash>,
    /// Hash of the config schema JSON file.
    pub(crate) schema: Hash,
}

/// A signature payload settings object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Settings {
    /// Hash of the settings schema JSON file.
    pub(crate) schema: Hash,
}

// impl SignaturePayload {
//     /// WASM module signature payload JSON schema.
//     const SIGNATURE_PAYLOAD_SCHEMA: &'static str =
//         include_str!("../../../../schemas/hermes_module_manifest.schema.json");

//     /// Create `SignaturePayload` from reader.
//     pub(crate) fn from_reader(reader: impl Read) -> anyhow::Result<()> {
//         let validator = SchemaValidator::from_str(Self::SIGNATURE_PAYLOAD_SCHEMA)?;

//         Ok(())
//     }
// }
