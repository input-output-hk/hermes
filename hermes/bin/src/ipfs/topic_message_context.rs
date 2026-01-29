//! A context attached to every IPFS message

use std::sync::{Arc, Mutex};

use catalyst_types::smt::Tree;

use crate::{
    app::ApplicationName,
    runtime_extensions::hermes::doc_sync::{self},
    wasm::module::ModuleId,
};

/// IPFS topic message context
#[derive(Clone)]
pub(super) struct TopicMessageContext {
    /// SMT.
    tree: Option<Arc<Mutex<Tree<doc_sync::Cid>>>>,
    /// Application name.
    app_name: ApplicationName,
    /// Module IDs
    module_ids: Option<Vec<ModuleId>>,
}

impl TopicMessageContext {
    /// Creates a new `TopicMessageContext`
    pub(crate) fn new(
        tree: Option<Arc<Mutex<Tree<doc_sync::Cid>>>>,
        app_name: ApplicationName,
        module_ids: Option<Vec<ModuleId>>,
    ) -> Self {
        Self {
            tree,
            app_name,
            module_ids,
        }
    }

    /// Returns the module IDs
    pub(super) fn module_ids(&self) -> Option<&Vec<ModuleId>> {
        self.module_ids.as_ref()
    }

    /// Returns the SMT
    pub(super) fn tree(&self) -> Option<&Arc<Mutex<Tree<doc_sync::Cid>>>> {
        self.tree.as_ref()
    }

    /// Returns the application name
    pub(super) fn app_name(&self) -> &ApplicationName {
        &self.app_name
    }
}
