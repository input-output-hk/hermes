//! A RBAC role data map.

use std::collections::HashMap;

use catalyst_types::catalyst_id::role_index::RoleId;
use serde::{Serialize, Serializer};

use crate::service::api::registration_get::v1::role_data::RbacRoleData;

/// A RBAC role data map.
#[derive(Debug, Clone)]
pub(crate) struct RoleMap(HashMap<RoleId, RbacRoleData>);

impl From<HashMap<RoleId, RbacRoleData>> for RoleMap {
    fn from(value: HashMap<RoleId, RbacRoleData>) -> Self {
        Self(value)
    }
}

impl Serialize for RoleMap {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(self.0.len()))?;

        for (key, val) in &self.0 {
            let key_str = key.to_string();
            map.serialize_entry(&key_str, val)?;
        }

        map.end()
    }
}
