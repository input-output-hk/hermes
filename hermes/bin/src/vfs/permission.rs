//! Permissions state management of the Hermes virtual file system.

#![allow(dead_code)]

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::utils::parse_path;

/// Permission level type.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum PermissionLevel {
    /// Read only permission level.
    Read,
    /// Read and write permission level.
    #[default]
    ReadAndWrite,
}

/// VFS permissions state stored in the radix tree structure, where the
/// each node relates to a single path element with the permission level.
struct PermissionsTree {
    /// Tree's root node.
    root: PermissionNodeRef,
}

/// `PermissionsTree` node type.
#[derive(Default)]
struct PermissionNode {
    /// Node permission level.
    permission: PermissionLevel,
    /// Node childs.
    childs: HashMap<String, PermissionNodeRef>,
}

/// Convinient type of the referenced `PermissionNode`.
type PermissionNodeRef = Rc<RefCell<PermissionNode>>;

impl PermissionsTree {
    /// Creates a new `PermissionsTree` instance with the root node which releates to the
    /// "/" path with the `PermissionLevel::ReadAndWrite` default permission level.
    fn new() -> Self {
        let root = PermissionNodeRef::default();
        Self { root }
    }

    /// Adds a new path to the `PermissionsTree` with the provided permission level.
    fn add_permission(&mut self, path: &str, permission: PermissionLevel) {
        let path_elements = parse_path(path);

        let mut walk = self.root.clone();
        for path_element in path_elements {
            let node = walk.clone();
            let mut node = node.borrow_mut();
            if let Some(child_node) = node.childs.get(&path_element) {
                walk = child_node.clone();
            } else {
                let new_node = PermissionNodeRef::default();
                node.childs.insert(path_element, new_node.clone());
                walk = new_node;
            }
        }
        // Update the last node with the provided permission
        walk.borrow_mut().permission = permission;
    }

    /// Gets the permission level for the provided path.
    fn get_permission(&self, path: &str) -> PermissionLevel {
        let mut permission = self.root.borrow().permission;

        let path_elements = parse_path(path);

        let mut walk = self.root.clone();
        for path_element in path_elements {
            let node = walk.clone();
            let node = node.borrow();
            if let Some(child_node) = node.childs.get(&path_element) {
                permission = child_node.borrow().permission;
                walk = child_node.clone();
            } else {
                break;
            }
        }

        permission
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_tree_test() {
        let mut tree = PermissionsTree::new();

        assert_eq!(tree.get_permission(""), PermissionLevel::ReadAndWrite);
        assert_eq!(tree.get_permission("/a/b"), PermissionLevel::ReadAndWrite);
        assert_eq!(tree.get_permission(r"\a\b"), PermissionLevel::ReadAndWrite);

        tree.add_permission("/a/b", PermissionLevel::Read);
        tree.add_permission("c/b", PermissionLevel::Read);

        assert_eq!(tree.get_permission("a"), PermissionLevel::ReadAndWrite);
        assert_eq!(tree.get_permission("a/b"), PermissionLevel::Read);
        assert_eq!(tree.get_permission("c"), PermissionLevel::ReadAndWrite);
        assert_eq!(tree.get_permission("c/b"), PermissionLevel::Read);
    }
}
