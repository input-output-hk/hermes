//! Hermes runtime extensions key traits (Event Initialization).
//!
//! Runtime extensions must implement these traits if they require code to execute for
//! resource management or other purposes at various phases of the node and application
//! life cycle.
use std::sync::LazyLock;

use dashmap::DashSet;
use keyed_lock::sync::KeyedLock;
use tracing::{Level, error, span};

use crate::{
    run_init_fini,
    runtime_context::HermesRuntimeContext,
    runtime_extensions::init::{errors::RteInitResult, priority::RteInitPriority},
};

/// Runtime Extension needs Initialization during Event Dispatch by the Node.
///
/// *MUST* be used with:
///
/// ```ignore
/// #[traitreg::register]
/// impl RteInitModule for MyRte {
///   // implementation goes here
/// }
/// ```
pub(crate) trait RteInitEvent {
    /// Initialize the Runtime extension at Event Dispatch.
    /// If it errors, the node does not crash, but the App will not
    /// load.
    ///
    /// Note: `self` is not required to be used by the implemented function.
    /// It is required because of the dynamic initialization logic.
    ///
    /// If `self` is used, it is consumed.  This allows the underlying state
    /// to be stored in a global static, or the like.
    #[allow(
        unused_variables,
        reason = "Preserves the API structure for better documentation."
    )]
    fn init(
        self: Box<Self>,
        ctx: &HermesRuntimeContext,
    ) -> RteInitResult {
        Ok(())
    }

    /// Returns the priority to be used when calling `init` and `fini`.
    ///
    /// IF the priority is the same, the order of initialization is non-deterministic.
    /// Higher priorities run before lower ones.
    ///
    /// Two values are returned
    ///
    /// 0 is the default priority, and should be used for most runtime extensions that do
    /// not care about the order of initialization.
    fn priority(
        &self,
        init: bool,
    ) -> i32 {
        RteInitPriority::default().priority(init)
    }

    /// Finalize the Runtime extension when the event is complete.
    /// Can Error, but will not cause the Node to terminate.
    ///
    /// Note: `self` is not required to be used by the implemented function.
    /// It is required because of the dynamic initialization logic.
    ///
    /// If `self` is used, it is consumed.  This allows the underlying state
    /// to be stored in a global static, or the like.
    #[allow(
        unused_variables,
        reason = "Preserves the API structure for better documentation."
    )]
    fn fini(
        self: Box<Self>,
        ctx: &HermesRuntimeContext,
    ) -> RteInitResult {
        Ok(())
    }
}

#[traitreg::registry(RteInitEvent)]
static RTE_INIT_EVENT_REGISTRY: () = ();

/// Locks the Initialized `DashSet` to stop race conditions, but only for specific apps.
/// TODO (SJ): Replace with a true key locked map.
/// <https://github.com/input-output-hk/catalyst-internal-docs/issues/39#issuecomment-3240936500>
static IS_RTE_EVENT_LOCK: LazyLock<KeyedLock<String>> = LazyLock::new(KeyedLock::<String>::new);

/// Is the RTE Initialized
static IS_RTE_EVENT_INITIALIZED: LazyLock<DashSet<String>> = LazyLock::new(DashSet::new);

/// How all Module Init is called.
///
/// Note: We use the same trait as the runtime extensions to enforce
/// uniformity in the call, but this struct IS NOT, and MUST NEVER BE
/// placed in the registry.
///
/// This is how the node interacts with the RTE App Initialization.
pub(crate) struct RteEvent;

impl RteEvent {
    /// Create a new instance of the `RteApp` being initialized.
    #[allow(
        clippy::unnecessary_box_returns,
        reason = "Its not unnecessary, the init and fini need it boxed."
    )]
    pub fn new() -> Box<Self>
    where Self: std::marker::Sized {
        Box::new(Self)
    }

    /// Create a unique lock ID for an App/Module pair
    fn lock_id(ctx: &HermesRuntimeContext) -> String {
        format!(
            "{}-{}-{}-{}",
            ctx.app_name(),
            ctx.module_id(),
            ctx.event_name(),
            ctx.exc_counter()
        )
        .to_string()
    }
}

impl RteInitEvent for RteEvent {
    fn init(
        self: Box<Self>,
        ctx: &HermesRuntimeContext,
    ) -> RteInitResult {
        let lock_id = Self::lock_id(ctx);

        // Prevents init or fini running simultaneously for the same app.
        let _guard = IS_RTE_EVENT_LOCK.lock(lock_id.clone());

        if !IS_RTE_EVENT_INITIALIZED.insert(lock_id) {
            error!(
                name=%ctx.app_name(),
                module=%ctx.module_id(),
                event=%ctx.event_name(),
                exc_count=%ctx.exc_counter(),
                "Multiple attempts to initialize event..  This does not cause problems, but don't do it.");
            return Ok(()); // Not an error which should stop us running.
        }

        run_init_fini!(
            init = true,
            registry = RTE_INIT_EVENT_REGISTRY,
            rte_trait = RteInitEvent,
            span_label = "Runtime Extension Event Initialization Span",
            (ctx)
        )
    }

    fn fini(
        self: Box<Self>,
        ctx: &HermesRuntimeContext,
    ) -> RteInitResult {
        let lock_id = Self::lock_id(ctx);

        // Prevents init or fini running simultaneously for the same app.
        let _guard = IS_RTE_EVENT_LOCK.lock(lock_id.clone());

        if IS_RTE_EVENT_INITIALIZED.remove(&lock_id).is_none() {
            error!(
                name=%ctx.app_name(),
                module=%ctx.module_id(),
                event=%ctx.event_name(),
                exc_count=%ctx.exc_counter(),
                "Multiple attempts to finalize event (or event never initialized).  This does not cause problems, but don't do it.");
            return Ok(()); // Not an error which should stop us running.
        }

        run_init_fini!(
            init = false,
            registry = RTE_INIT_EVENT_REGISTRY,
            rte_trait = RteInitEvent,
            span_label = "Runtime Extension Event Finalization Span",
            (ctx)
        )
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::RTE_INIT_EVENT_REGISTRY;

    /// Tests that a new `RuntimeExtensionErrors` is empty
    #[test]
    fn test_all_registered_apps_have_constructors() {
        for registered in RTE_INIT_EVENT_REGISTRY.iter() {
            // Check all registered App Initializers have constructors.
            assert!(
                registered.instanciate().is_some(),
                "Missing Constructor in the registered runtime extension [ name:{} - path:{} - file:{} - trait_name:{} - module_path:{} ]",
                registered.name(),
                registered.path(),
                registered.file(),
                registered.trait_name(),
                registered.module_path(),
            );
        }
    }
}
