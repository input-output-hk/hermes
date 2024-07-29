//! Permissions state management of the Hermes virtual file system.

#![allow(dead_code, missing_docs, clippy::missing_docs_in_private_items)]

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::utils::parse_path;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum PermissionLevel {
    Read,
    #[default]
    ReadAndWrite,
}

struct PermissionsTree {
    root: PermissionNodeRef,
}

#[derive(Default)]
struct PermissionNode {
    permission: PermissionLevel,
    childs: HashMap<String, PermissionNodeRef>,
}

type PermissionNodeRef = Rc<RefCell<PermissionNode>>;

impl PermissionsTree {
    fn new() -> Self {
        let root = PermissionNodeRef::default();
        Self { root }
    }

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
