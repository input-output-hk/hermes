//! Catalyst RBAC Security Scheme

use super::token::CatalystRBACTokenV1;

/// Catalyst RBAC Access Token
#[allow(clippy::module_name_repetitions)]
pub struct CatalystRBACSecurityScheme(CatalystRBACTokenV1);

impl From<CatalystRBACSecurityScheme> for CatalystRBACTokenV1 {
    fn from(value: CatalystRBACSecurityScheme) -> Self {
        value.0
    }
}
