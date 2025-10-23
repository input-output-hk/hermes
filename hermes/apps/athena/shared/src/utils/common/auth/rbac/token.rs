//! Catalyst RBAC Token utility functions.

// cspell: words rsplit Fftx

use std::{
    fmt::{Display, Formatter},
    sync::LazyLock,
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use base64::{prelude::BASE64_URL_SAFE_NO_PAD, Engine};
use cardano_blockchain_types::Network;
use catalyst_types::catalyst_id::CatalystId;
use chrono::{TimeDelta, Utc};
use ed25519_dalek::{ed25519::signature::Signer, Signature, SigningKey, VerifyingKey};
use rbac_registration::registration::cardano::RegistrationChain;
use regex::Regex;

/// Captures just the digits after last slash
/// This Regex should not fail
#[allow(clippy::unwrap_used)]
static REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"/\d+$").unwrap());

/// A Catalyst RBAC Authorization Token.
///
/// See [this document] for more details.
///
/// [this document]: https://github.com/input-output-hk/catalyst-voices/blob/main/docs/src/catalyst-standards/permissionless-auth/auth-header.md
#[derive(Debug, Clone)]
pub(crate) struct CatalystRBACTokenV1 {
    /// A Catalyst identifier.
    catalyst_id: CatalystId,
    /// A network value.
    ///
    /// The network value is contained in the Catalyst ID and can be accessed from it, but
    /// it is a string, so we convert it to this enum during the validation.
    network: Network,
    /// Ed25519 Signature of the Token
    signature: Signature,
    /// Raw bytes of the token without the signature.
    raw: Vec<u8>,
    /// A corresponded RBAC chain, constructed from the most recent data from the
    /// database. Lazy initialized
    reg_chain: Option<RegistrationChain>,
}

impl CatalystRBACTokenV1 {
    /// Bearer Token prefix for this token.
    const AUTH_TOKEN_PREFIX: &str = "catid.";

    /// Creates a new token instance.
    // TODO: Remove the attribute when the function is used.
    #[allow(dead_code)]
    pub(crate) fn new(
        network: &str,
        subnet: Option<&str>,
        role0_pk: VerifyingKey,
        sk: &SigningKey,
    ) -> Result<Self> {
        let catalyst_id = CatalystId::new(network, subnet, role0_pk)
            .with_nonce()
            .as_id();
        let network = convert_network(&catalyst_id.network())?;
        let raw = as_raw_bytes(&catalyst_id.to_string());
        let signature = sk.sign(&raw);

        Ok(Self {
            catalyst_id,
            network,
            signature,
            raw,
            reg_chain: None,
        })
    }

    /// Parses a token from the given string.
    ///
    /// The token consists of the following parts:
    /// - "catid" prefix.
    /// - Nonce.
    /// - Network.
    /// - Role 0 public key.
    /// - Signature.
    ///
    /// For example:
    /// ```text
    /// catid.:173710179@preprod.cardano/FftxFnOrj2qmTuB2oZG2v0YEWJfKvQ9Gg8AgNAhDsKE.<signature>
    /// ```
    pub(crate) fn parse(token: &str) -> Result<CatalystRBACTokenV1> {
        let token = token
            .strip_prefix(Self::AUTH_TOKEN_PREFIX)
            .ok_or_else(|| anyhow!("Missing token prefix"))?;
        let (token, signature) = token
            .rsplit_once('.')
            .ok_or_else(|| anyhow!("Missing token signature"))?;
        let signature = BASE64_URL_SAFE_NO_PAD
            .decode(signature.as_bytes())
            .context("Invalid token signature encoding")?
            .try_into()
            .map(|b| Signature::from_bytes(&b))
            .map_err(|_| anyhow!("Invalid token signature length"))?;
        let raw = as_raw_bytes(token);

        let catalyst_id: CatalystId = token.parse().context("Invalid Catalyst ID")?;
        if catalyst_id.username().is_some_and(|n| !n.is_empty()) {
            return Err(anyhow!("Catalyst ID must not contain username"));
        }
        if !catalyst_id.clone().is_id() {
            return Err(anyhow!("Catalyst ID must be in an ID format"));
        }
        if catalyst_id.nonce().is_none() {
            return Err(anyhow!("Catalyst ID must have nonce"));
        }

        if REGEX.is_match(token) {
            return Err(anyhow!(
                "Catalyst ID mustn't have role or rotation specified"
            ));
        }
        let network = convert_network(&catalyst_id.network())?;

        Ok(Self {
            catalyst_id,
            network,
            signature,
            raw,
            reg_chain: None,
        })
    }

    /// Given the `PublicKey`, verifies the token was correctly signed.
    pub(crate) fn verify(
        &self,
        public_key: &VerifyingKey,
    ) -> Result<()> {
        public_key
            .verify_strict(&self.raw, &self.signature)
            .context("Token signature verification failed")
    }

    /// Checks that the token timestamp is valid.
    ///
    /// The timestamp is valid if it isn't too old or too skewed.
    pub(crate) fn is_young(
        &self,
        max_age: Duration,
        max_skew: Duration,
    ) -> bool {
        let Some(token_age) = self.catalyst_id.nonce() else {
            return false;
        };

        let now = Utc::now();

        // The token is considered old if it was issued more than max_age ago.
        // And newer than an allowed clock skew value
        // This is a safety measure to avoid replay attacks.
        let Ok(max_age) = TimeDelta::from_std(max_age) else {
            return false;
        };
        let Ok(max_skew) = TimeDelta::from_std(max_skew) else {
            return false;
        };
        let Some(min_time) = now.checked_sub_signed(max_age) else {
            return false;
        };
        let Some(max_time) = now.checked_add_signed(max_skew) else {
            return false;
        };
        (min_time < token_age) && (max_time > token_age)
    }

    /// Returns a Catalyst ID from the token.
    pub(crate) fn catalyst_id(&self) -> &CatalystId {
        &self.catalyst_id
    }

    /// Returns a network.
    #[allow(dead_code)]
    pub(crate) fn network(&self) -> Network {
        self.network
    }
}

impl Display for CatalystRBACTokenV1 {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "{}{}.{}",
            CatalystRBACTokenV1::AUTH_TOKEN_PREFIX,
            self.catalyst_id,
            BASE64_URL_SAFE_NO_PAD.encode(self.signature.to_bytes())
        )
    }
}

/// Converts the given token string to raw bytes.
fn as_raw_bytes(token: &str) -> Vec<u8> {
    // The signature is calculated over all bytes in the token including the final '.'.
    CatalystRBACTokenV1::AUTH_TOKEN_PREFIX
        .bytes()
        .chain(token.bytes())
        .chain(".".bytes())
        .collect()
}

/// Checks if the given network is supported.
fn convert_network((network, subnet): &(String, Option<String>)) -> Result<Network> {
    if network != "cardano" {
        return Err(anyhow!("Unsupported network: {network}"));
    }

    match subnet.as_deref() {
        None => Ok(Network::Mainnet),
        Some("preprod") => Ok(Network::Preprod),
        Some("preview") => Ok(Network::Preview),
        Some(subnet) => Err(anyhow!("Unsupported host: {subnet}.{network}",)),
    }
}
