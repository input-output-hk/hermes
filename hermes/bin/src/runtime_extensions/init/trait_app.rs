//! Hermes runtime extensions key traits (Application Pre Initialization).
//!
//! Runtime extensions must implement these traits if they require code to execute for
//! resource management or other purposes at various phases of the node and application
//! life cycle.
use std::sync::LazyLock;

use dashmap::DashSet;
use keyed_lock::sync::KeyedLock;
use tracing::{error, span, Level};

use crate::{
    app::ApplicationName,
    run_init_fini,
    runtime_extensions::init::{errors::RteInitResult, executor, priority::RteInitPriority},
};

/// Runtime Extension needs Initialization before an Application is loaded by the Node.
///
/// *MUST* be used with:
///
/// ```ignore
/// #[traitreg::register]
/// impl RteInitApp for MyRte {
///   // implementation goes here
/// }
/// ```
pub(crate) trait RteInitApp {
    /// Initialize the Runtime extension at App startup (before any modules are
    /// initialized). If it errors, the node does not crash, but the App will not
    /// load.
    ///
    /// Note: `self` is not required to be used by the implemented function.
    /// It is required because of the dynamic initialization logic.
    ///
    /// If `self` is used, it is consumed.  This allows the underlying state
    /// to be stored in a global static, or the like.
    fn init(
        self: Box<Self>,
        #[allow(
            unused_variables,
            reason = "Preserves the API structure for better documentation."
        )]
        name: &ApplicationName,
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

    /// Finalize the Runtime extension when the application is fully stopped.
    /// Can Error, but will not cause the Node to terminate.
    ///
    /// Note: `self` is not required to be used by the implemented function.
    /// It is required because of the dynamic initialization logic.
    ///
    /// If `self` is used, it is consumed.  This allows the underlying state
    /// to be stored in a global static, or the like.
    fn fini(
        self: Box<Self>,
        #[allow(
            unused_variables,
            reason = "Preserves the API structure for better documentation."
        )]
        name: &ApplicationName,
    ) -> RteInitResult {
        Ok(())
    }
}

#[traitreg::registry(RteInitApp)]
static RTE_INIT_APP_REGISTRY: () = ();

/// Locks the Initialized `DashSet` to stop race conditions, but only for specific apps.
static IS_RTE_APP_LOCK: LazyLock<KeyedLock<String>> = LazyLock::new(KeyedLock::<String>::new);

/// Is the RTE Initialized
static IS_RTE_APP_INITIALIZED: LazyLock<DashSet<ApplicationName>> = LazyLock::new(DashSet::new);

/// How all Application Init is called.
///
/// Note: We use the same trait as the runtime extensions to enforce
/// uniformity in the call, but this struct IS NOT, and MUST NEVER BE
/// placed in the registry.
///
/// This is how the node interacts with the RTE App Initialization.
pub(crate) struct RteApp;

impl RteApp {
    /// Create a new instance of the `RteApp` being initialized.
    #[allow(
        clippy::unnecessary_box_returns,
        reason = "Its not unnecessary, the init and fini need it boxed."
    )]
    pub fn new() -> Box<Self>
    where Self: std::marker::Sized {
        Box::new(Self)
    }
}

impl RteInitApp for RteApp {
    fn init(
        self: Box<Self>,
        name: &ApplicationName,
    ) -> RteInitResult {
        // Prevents init or fini running simultaneously for the same app.
        let _guard = IS_RTE_APP_LOCK.lock(name.to_string());

        if !IS_RTE_APP_INITIALIZED.insert(name.clone()) {
            error!(name=%name,"Multiple attempts to initialize application..  This does not cause problems, but don't do it.");
            return Ok(()); // Not an error which should stop us running.
        }

        let errors = run_init_fini!(
            init = true,
            registry = RTE_INIT_APP_REGISTRY,
            rte_trait = RteInitApp,
            span_label = "Runtime Extension Node Initialization Span",
            (name)
        );

        errors
    }

    fn fini(
        self: Box<Self>,
        name: &ApplicationName,
    ) -> RteInitResult {
        // Prevents init or fini running simultaneously for the same app.
        let _guard = IS_RTE_APP_LOCK.lock(name.to_string());

        if IS_RTE_APP_INITIALIZED.remove(name).is_none() {
            error!(name=%name,"Multiple attempts to finalize application (or application never initialized).  This does not cause problems, but don't do it.");
            return Ok(()); // Not an error which should stop us running.
        }

        let errors = run_init_fini!(
            init = true,
            registry = RTE_INIT_APP_REGISTRY,
            rte_trait = RteInitApp,
            span_label = "Runtime Extension Node Initialization Span",
            (name)
        );

        errors
    }
}
