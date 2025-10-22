//! Either has No Authorization, or RBAC Token.

// use headers::{authorization::Bearer, Authorization, HeaderMapExt};

use super::{
    none::NoAuthorization,
    rbac::{scheme::CatalystRBACSecurityScheme, token::CatalystRBACTokenV1},
};

#[allow(dead_code, clippy::upper_case_acronyms, clippy::large_enum_variant)]
/// Endpoint allows Authorization with or without RBAC Token.
pub enum NoneOrRBAC {
    /// Has RBAC Token.
    RBAC(CatalystRBACSecurityScheme),
    /// Has No Authorization.
    None(NoAuthorization),
}

impl From<NoneOrRBAC> for Option<CatalystRBACTokenV1> {
    fn from(value: NoneOrRBAC) -> Self {
        match value {
            NoneOrRBAC::RBAC(auth) => Some(auth.into()),
            NoneOrRBAC::None(_) => None,
        }
    }
}
