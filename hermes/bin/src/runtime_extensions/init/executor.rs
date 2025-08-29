//! runtime extension initialization executor functions and macros.

use traitreg::RegisteredImplWrapper;

/// Generic data for sorting initialization/finalization of runtime extensions.
pub(crate) struct InstanceData<'a, T: ?Sized + 'static> {
    /// RTE Registration Data
    pub register: &'a RegisteredImplWrapper<Box<T>>,
    /// RTE Instance
    pub instance: Box<T>,
}

/// Helper to make running `init` anf `fini` for a Trait registry easy and consistent.
#[macro_export]
macro_rules! run_init_fini {
    (
        init = $init:expr,
        registry = $registry:ident,
        rte_trait = $rte_trait:ident,
        span_label = $span_label:expr,
        ( $( $arg:expr ),* $(,)? )
    ) => {{
        use $crate::runtime_extensions::init::errors::RuntimeExtensionErrors;

        /// Data we use to help sort initialization and finalization of runtime extensions.
        type InstanceData<'a> = executor::InstanceData<'a, dyn $rte_trait>;

        let errors = RuntimeExtensionErrors::new();

        // TODO: (SJ) Creating the list of sorted (and grouped) instances could in theory
        // be done only once, and saved inside a OnceLock, however, the instances are not
        // Send + Sync, so this currently isn't possible as they can't be moved between threads.
        // Would require an update to the traitreg crate most likely.
        // Suggestion would be to make creating any instance wrapped in an Arc<Box<>> perhaps.
        // Needs more investigation.
        // For now, this is enough, if the number of registered runtime extensions grows,
        // or the work they need to do gets more complex, then this can be revisited.
        let mut instances: Vec<InstanceData> = Vec::new();

        for registered in $registry.iter() {
            // First we collect all the instances.
            if let Some(instance) = registered.instanciate() {
                instances.push(InstanceData {
                    register: registered,
                    instance,
                });
            } else {
                error!(
                    name = registered.name(),
                    path = registered.path(),
                    file = registered.file(),
                    trait_name = registered.trait_name(),
                    module_path = registered.module_path(),
                    "Failed to create an instance of the runtime extension for runtime initialization."
                );
            }
        }

        instances.sort_by_key(|rte| std::cmp::Reverse(rte.instance.priority($init)));

        for rte in instances {
            // TODO: (SJ) This could be executed in parallel.
            // For example, instead of a sorted list, we used a sorted list of groups with the
            // same priorities.  The individual groups can be executed sequentially, with
            // the runtime extensions at the same priority being executed in parallel.
            // this would likely improve performance, especially for the event dispatch initializer/finalizer.
            // However, it isn't currently possible because the instances from the registry are not Send + Sync
            // so they can't be moved between threads.
            // This would also need the grouped and sorted list of runtime extensions to be held in a once
            // lock to eliminate the requirement to constantly re-calculate them.
            // Would most likely require an update/fork of the `traitreg` crate to make instances thread safe.
            // Currently this is not a big problem, but could become bigger if the runtime extension event
            // initialization became bigger or more complex.  Runtime, App and Module initialization are not time
            // sensitive, so would not be hurt, but would likely not help performance to be parallel executed to
            // any noticeable degree.
            span!(
                Level::DEBUG,
                $span_label,
                priority = rte.instance.priority($init),
                name = rte.register.name(),
                path = rte.register.path(),
                file = rte.register.file(),
                trait_name = rte.register.trait_name(),
                module_path = rte.register.module_path(),
            )
            .in_scope(|| {
                if $init {
                errors.maybe(rte.instance.init( $( $arg ),* ));
                } else {
                errors.maybe(rte.instance.fini( $( $arg ),* ));

                }
            });
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }};
}
