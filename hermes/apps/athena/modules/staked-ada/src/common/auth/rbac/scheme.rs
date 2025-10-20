//! Catalyst RBAC Security Scheme

use super::token::CatalystRBACTokenV1;

/// Catalyst RBAC Access Token
// #[derive(SecurityScheme)]
// #[oai(
//     ty = "bearer",
//     key_name = "Authorization", // MUST match the `AUTHORIZATION_HEADER` constant.
//     bearer_format = "catalyst-rbac-token",
//     checker = "checker_api_catalyst_auth"
// )]
#[allow(clippy::module_name_repetitions)]
pub(crate) struct CatalystRBACSecurityScheme(CatalystRBACTokenV1);

impl From<CatalystRBACSecurityScheme> for CatalystRBACTokenV1 {
    fn from(value: CatalystRBACSecurityScheme) -> Self {
        value.0
    }
}
