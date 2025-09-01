//! Hermes runtime extensions key traits.
//!
//! Runtime extensions must implement these traits if they require code to execute for
//! resource management or other purposes at various phases of the node and application
//! life cycle.

use std::sync::{LazyLock, Mutex};

use tracing::{error, span, Level};

use crate::{
    add_rte_error, run_init_fini,
    runtime_extensions::init::{
        errors::{RteInitResult, RuntimeExtensionErrors},
        metadata::RteMetadata,
        priority::RteInitPriority,
    },
};

/// Runtime Extension needs Initialization at Node Startup
///
/// *MUST* be used with:
///
/// ```ignore
/// #[traitreg::register(new)]
/// impl RteInitRuntime for MyRte {
///   // implementation goes here
/// }
/// ```
pub(crate) trait RteInitRuntime {
    /// Initialize the Runtime extension at node startup.
    /// This SHOULD be infallible, but if it does fail, then the node must
    /// present the errors reported to the user in a nice and actionable way BEFORE
    /// it exits.
    ///
    /// Note: `self` is not required to be used by the implemented function.
    /// It is required because of the dynamic initialization logic.
    ///
    /// If `self` is used, it is consumed.  This allows the underlying state
    /// to be stored in a global static, or the like.
    fn init(self: Box<Self>) -> RteInitResult {
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

    /// Initialize the Runtime extension at node startup.
    /// Is infallible, because runtime extensions should not fail to initialize.
    ///
    /// Note: `self` is not required to be used by the implemented function.
    /// It is required because of the dynamic initialization logic.
    ///
    /// If `self` is used, it is consumed.  This allows the underlying state
    /// to be stored in a global static, or the like.
    fn fini(self: Box<Self>) -> RteInitResult {
        Ok(())
    }
}

#[traitreg::registry(RteInitRuntime)]
static RTE_INIT_RUNTIME_REGISTRY: () = ();

/// Is the RTE Initialized
static IS_RTE_RUNTIME_INITIALIZED: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));
/// Is the RTE finalized
static IS_RTE_RUNTIME_FINALIZED: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));

/// How all RTE Runtime Init is called.
///
/// Note: We use the same trait as the runtime extensions to enforce
/// uniformity in the call, but this struct IS NOT, and MUST NEVER BE
/// placed in the registry.
///
/// This is how the node interacts with the RTE Runtime Initialization.
pub(crate) struct RteRuntime;

impl RteRuntime {
    /// Create a new instance of the `RteRuntime` being initialized.
    #[allow(
        clippy::unnecessary_box_returns,
        reason = "Its not unnecessary, the init and fini need it boxed."
    )]
    pub fn new() -> Box<Self>
    where Self: std::marker::Sized {
        Box::new(Self)
    }
}

impl RteInitRuntime for RteRuntime {
    fn init(self: Box<Self>) -> RteInitResult {
        match IS_RTE_RUNTIME_INITIALIZED.lock() {
            Ok(mut initialized) => {
                if *initialized {
                    error!("Multiple calls to RTE Node `init()`.  This does not cause problems, but don't do it.");
                    return Ok(()); // Not an error which should stop us running.
                }

                let errors = run_init_fini!(
                    init = true,
                    registry = RTE_INIT_RUNTIME_REGISTRY,
                    rte_trait = RteInitRuntime,
                    span_label = "Runtime Extension Node Initialization Span",
                    ()
                );

                *initialized = true;

                errors
            },
            Err(e) => {
                let msg = "Poisoned `IS_RTE_RUNTIME_INITIALIZED` on RTE Runtime Initialization. Should never happen.";
                error!(
                    error = ?e,
                    "Failed to acquire lock on RTE Runtime Initialization. Should never happen."
                );

                let errors = RuntimeExtensionErrors::new();
                add_rte_error!(errors, RteMetadata::none(), ImpossibleError {
                    description: msg.to_string(),
                });
                Err(errors)
            },
        }
    }

    fn fini(self: Box<Self>) -> RteInitResult {
        // Needs to be fully initialized or we won't run finalize.
        match IS_RTE_RUNTIME_INITIALIZED.lock() {
            Ok(initialized) => {
                if !*initialized {
                    error!("RTE Node `fini()` called but runtimes are not initialized.  This does not cause problems by itself, but you probably did something very wrong.");
                    return Ok(()); // Not an error which stops us ending ok.
                }

                match IS_RTE_RUNTIME_FINALIZED.lock() {
                    Ok(mut finalized) => {
                        if *finalized {
                            error!("`runtime_fini()` called multiple times.  This does not cause problems by itself, but you probably did something very wrong.");
                            return Ok(()); // Not fatal, but still wrong.
                        }

                        let errors = run_init_fini!(
                            init = false,
                            registry = RTE_INIT_RUNTIME_REGISTRY,
                            rte_trait = RteInitRuntime,
                            span_label = "Runtime Extension Node Finalization Span",
                            ()
                        );

                        *finalized = true;
                        errors
                    },
                    Err(e) => {
                        let msg = "Poisoned `IS_RTE_RUNTIME_FINALIZED` on RTE Runtime Finalization. Should never happen.";

                        error!(
                            error = ?e,
                            msg
                        );

                        let errors = RuntimeExtensionErrors::new();
                        add_rte_error!(errors, RteMetadata::none(), ImpossibleError {
                            description: msg.to_string(),
                        });
                        Err(errors)
                    },
                }
            },
            Err(e) => {
                let msg = "Poisoned `IS_RTE_RUNTIME_INITIALIZED` on RTE Runtime Finalization. Should never happen.";
                error!(
                    error = ?e,
                    msg
                );

                let errors = RuntimeExtensionErrors::new();
                add_rte_error!(errors, RteMetadata::none(), ImpossibleError {
                    description: msg.to_string(),
                });
                Err(errors)
            },
        }
    }
}
