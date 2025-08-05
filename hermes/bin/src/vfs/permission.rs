//! Permissions state management of the Hermes virtual file system.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use dashmap::DashMap;

use crate::utils::parse_path;

/// Permission level type.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PermissionLevel {
    /// Read only permission level.
    Read,
    /// Read and write permission level.
    #[default]
    ReadAndWrite,
}

impl From<PermissionLevel> for bool {
    fn from(level: PermissionLevel) -> Self {
        match level {
            PermissionLevel::Read => false,
            PermissionLevel::ReadAndWrite => true,
        }
    }
}

impl From<bool> for PermissionLevel {
    fn from(level: bool) -> Self {
        if level {
            PermissionLevel::ReadAndWrite
        } else {
            PermissionLevel::Read
        }
    }
}

/// VFS permissions state stored in the radix tree structure, where the
/// each node relates to a single path element with the permission level.
/// `PermissionLevel::ReadAndWrite` is a default permission level so if permission was not
/// defined for the path explicitly `PermissionsState` will return the
/// `PermissionLevel::ReadAndWrite` permission level for the asked path.
#[derive(Debug)]
pub(crate) struct PermissionsState {
    /// Tree's root node.
    root: PermissionNodeRef,
}

/// `PermissionsTree` node type.
#[derive(Debug)]
struct PermissionNode {
    /// Node permission level.
    permission: AtomicBool,
    /// Node childs.
    childs: DashMap<String, PermissionNodeRef>,
}

impl Default for PermissionNode {
    fn default() -> Self {
        Self {
            permission: AtomicBool::new(PermissionLevel::default().into()),
            childs: DashMap::new(),
        }
    }
}

/// Convenient type of the referenced `PermissionNode`.
type PermissionNodeRef = Arc<PermissionNode>;

impl PermissionsState {
    /// Creates a new `PermissionsTree` instance with the root node which relates to the
    /// "/" path with the `PermissionLevel::ReadAndWrite` default permission level.
    pub(crate) fn new() -> Self {
        let root = PermissionNodeRef::default();
        Self { root }
    }

    /// Adds a new path to the `PermissionsTree` with the provided permission level.
    pub(crate) fn add_permission(
        &mut self,
        path: &str,
        permission: PermissionLevel,
    ) {
        let path_elements = parse_path(path);

        let mut walk = self.root.clone();
        for path_element in path_elements {
            let node = walk.clone();
            if let Some(child_node) = node.childs.get(&path_element) {
                walk = child_node.value().clone();
            } else {
                let new_node = PermissionNodeRef::default();
                node.childs.insert(path_element, new_node.clone());
                walk = new_node;
            };
        }
        // Update the last node with the provided permission
        walk.permission.store(permission.into(), Ordering::Release);
    }

    /// Gets the permission level for the provided path.
    pub(crate) fn get_permission(
        &self,
        path: &str,
    ) -> PermissionLevel {
        let mut permission: PermissionLevel = self.root.permission.load(Ordering::Acquire).into();

        let path_elements = parse_path(path);

        let mut walk = self.root.clone();
        for path_element in path_elements {
            let node = walk.clone();
            if let Some(child_node) = node.childs.get(&path_element) {
                permission = child_node.permission.load(Ordering::Acquire).into();

                if permission == PermissionLevel::Read {
                    break;
                }

                walk = child_node.clone();
            } else {
                break;
            };
        }

        permission
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_level_test() {
        assert!(!Into::<bool>::into(PermissionLevel::Read));
        assert!(Into::<bool>::into(PermissionLevel::ReadAndWrite));

        assert_eq!(Into::<PermissionLevel>::into(false), PermissionLevel::Read);
        assert_eq!(
            Into::<PermissionLevel>::into(true),
            PermissionLevel::ReadAndWrite
        );
    }

    #[test]
    fn permission_tree_test() {
        let mut tree = PermissionsState::new();

        assert_eq!(tree.get_permission(""), PermissionLevel::ReadAndWrite);
        assert_eq!(tree.get_permission("/a/b"), PermissionLevel::ReadAndWrite);

        tree.add_permission("/a/b", PermissionLevel::Read);
        tree.add_permission("c/b", PermissionLevel::Read);

        assert_eq!(tree.get_permission("a"), PermissionLevel::ReadAndWrite);
        assert_eq!(tree.get_permission("a/b"), PermissionLevel::Read);
        assert_eq!(tree.get_permission("c"), PermissionLevel::ReadAndWrite);
        assert_eq!(tree.get_permission("c/b"), PermissionLevel::Read);

        tree.add_permission("/a/b", PermissionLevel::ReadAndWrite);
        tree.add_permission("a", PermissionLevel::Read);

        assert_eq!(tree.get_permission("a"), PermissionLevel::Read);
        assert_eq!(tree.get_permission("a/b"), PermissionLevel::Read);
    }
}
