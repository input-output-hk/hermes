//! A RBAC role data map.

/// A RBAC role data map.
#[derive(Debug, Clone)]
pub(crate) struct RoleMap(HashMap<RoleId, RbacRoleData>);

impl From<HashMap<RoleId, RbacRoleData>> for RoleMap {
    fn from(value: HashMap<RoleId, RbacRoleData>) -> Self {
        Self(value)
    }
}
