//! RBAC token validation logic.

use std::time::Duration;

use catalyst_types::catalyst_id::role_index::RoleId;
use rbac_registration::registration::cardano::RegistrationChain;
use shared::{
    bindings::hermes::cardano,
    utils::{log::warn, sqlite::open_db_connection},
};

use super::token::CatalystRBACTokenV1;
use crate::{
    api_keys::check_api_key,
    hermes::http_gateway::api::Headers,
    response::{AuthResponse, AuthTokenAccessViolation, AuthTokenError},
};

/// Time in the past the Token can be valid for.
const MAX_TOKEN_AGE: Duration = Duration::from_secs(60 * 60); // 1 hour.

/// Time in the future the Token can be valid for.
const MAX_TOKEN_SKEW: Duration = Duration::from_secs(5 * 60); // 5 minutes

/// [here]: https://github.com/input-output-hk/catalyst-voices/blob/main/docs/src/catalyst-standards/permissionless-auth/auth-header.md#backend-processing-of-the-token
pub fn checker_api_catalyst_auth(
    headers: &Headers,
    bearer_token: &str,
    network: cardano::api::CardanoNetwork,
) -> AuthResponse {
    // Step 1-5: Parse and validate token format
    let mut token = match CatalystRBACTokenV1::parse(bearer_token) {
        Ok(token) => token,
        Err(e) => {
            return AuthResponse::Unauthorized(AuthTokenError::ParseRbacToken(e.to_string()));
        },
    };

    // Step 6: Get the registration chain
    let reg_chain = match get_registration(network, &mut token) {
        Ok(chain) => chain,
        Err(e) => {
            return e;
        },
    };

    // Step 7: Verify that the nonce is in the acceptable range.
    // If `InternalApiKeyAuthorization` auth is provided, skip validation.
    if check_api_key(headers).is_err() && !token.is_young(MAX_TOKEN_AGE, MAX_TOKEN_SKEW) {
        // Token is too old or too far in the future.
        warn!("Auth token expired: {token}");
        return AuthResponse::Forbidden(AuthTokenAccessViolation(vec!["EXPIRED".to_string()]));
    }

    // Step 8: Get the latest stable signing certificate registered for Role 0.
    let Some((latest_pk, _)) = reg_chain.get_latest_signing_public_key_for_role(RoleId::Role0)
    else {
        warn!(
            "Unable to get last signing key for {} Catalyst ID",
            token.catalyst_id()
        );
        return AuthResponse::Unauthorized(AuthTokenError::LatestSigningKey);
    };

    // Step 9: Verify the signature against the Role 0 pk.

    if token.verify(&latest_pk).is_err() {
        warn!("Invalid signature for token: {token}");
        return AuthResponse::Forbidden(AuthTokenAccessViolation(vec![
            "INVALID SIGNATURE".to_string(),
        ]));
    }

    // Step 10 is optional and isn't currently implemented.
    //   - Get the latest unstable signing certificate registered for Role 0.
    //   - Verify the signature against the Role 0 Public Key and Algorithm identified by the
    //     certificate. If this fails, return 403.

    // Step 11: Token is valid
    AuthResponse::Ok
}

/// Get the registration chain.
fn get_registration(
    network: cardano::api::CardanoNetwork,
    token: &mut CatalystRBACTokenV1,
) -> anyhow::Result<RegistrationChain, AuthResponse> {
    let persistent = open_db_connection(false).map_err(|_| {
        AuthResponse::ServiceUnavailable("Failed to open persistent database".to_string())
    })?;

    let volatile = open_db_connection(false).map_err(|_| {
        AuthResponse::ServiceUnavailable("Failed to open volatile database".to_string())
    })?;

    let network_resource = cardano::api::Network::new(network).map_err(|e| {
        AuthResponse::ServiceUnavailable(format!("Failed to create network resource: {e}"))
    })?;

    match token.reg_chain(&persistent, &volatile, &network_resource) {
        Ok(Some(chain)) => Ok(chain),
        Ok(None) => {
            Err(AuthResponse::Unauthorized(
                AuthTokenError::RegistrationNotFound,
            ))
        },
        Err(e) => {
            Err(AuthResponse::Unauthorized(AuthTokenError::BuildRegChain(
                e.to_string(),
            )))
        },
    }
}
