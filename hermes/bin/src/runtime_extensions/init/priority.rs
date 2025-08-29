//! RTE Init Priority

/// Priorities for Runtime Extension Initialization and Finalization.
///
/// Higher priorities run before lower ones.
///
/// 0 = default priority, and priority can be negative to run after the defaults.
#[derive(Default)]
pub(crate) struct RteInitPriority {
    /// Initialization Priority.  Higher values init first.
    pub init: i32,
    /// Finalization Priority.  Higher values finalize first.
    pub fini: i32,
}

impl RteInitPriority {
    /// Get the priority if init is true or false.
    pub(crate) fn priority(
        &self,
        init: bool,
    ) -> i32 {
        if init {
            self.init
        } else {
            self.fini
        }
    }
}
