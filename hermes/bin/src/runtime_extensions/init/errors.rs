//! Initialization Finalization Errors.
//!
//! Note, while this is similar to `ProblemReport`,
//! it is strictly an Error encountered during initialization/finalization.
use std::{fmt::Display, sync::Arc};

use orx_concurrent_vec::ConcurrentVec;

use crate::runtime_extensions::init::metadata::RteMetadata;

/// Result type that is returned from all `init` and `fini` implementations.
pub(crate) type RteInitResult = Result<(), RuntimeExtensionErrors>;

/// All errors we encountered while running initialization or finalization of a runtime
/// extension.
#[derive(Clone, thiserror::Error, Debug)]
#[allow(dead_code)]
pub(crate) struct RuntimeExtensionErrors(pub Arc<ConcurrentVec<RuntimeExtensionError>>);

impl Display for RuntimeExtensionErrors {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        // TODO: (SJ) needs a reasonable Display implementation when we actually have code that
        // uses this error properly.
        f.write_str("[\n]")?;
        for err in self.0.iter() {
            f.write_fmt(format_args!("  {},\n", err.cloned()))?;
        }
        f.write_str("]")
    }
}

impl RuntimeExtensionErrors {
    /// Create a new set of Runtime Extension Errors
    #[allow(dead_code)]
    pub fn new() -> Self {
        RuntimeExtensionErrors(Arc::new(ConcurrentVec::new()))
    }

    /// Are there any errors present?
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// How many errors present?
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Extends the `RuntimeExtensionErrors` with another set of errors.
    /// This is how we accumulate errors from multiple runtime extensions.
    #[allow(dead_code)]
    #[allow(
        clippy::needless_pass_by_value,
        reason = "We intentionally do this because `other` should not be used after extension."
    )]
    pub fn extend(
        &self,
        other: Self,
    ) {
        for error in other.0.iter() {
            self.0.push(error.cloned());
        }
    }

    /// Push a new `RuntimeExtensionError` into the collection of errors.
    /// This is how we accumulate individual errors.
    #[allow(dead_code)]
    pub fn push(
        &self,
        #[allow(
            clippy::needless_pass_by_value,
            reason = "We intentionally do this because `other` should not be used after extension."
        )]
        error: RuntimeExtensionError,
    ) -> usize {
        self.0.push(error)
    }

    /// Push a new `RuntimeExtensionError` into the collection of errors.
    /// But ONLY if the status is an error.
    #[allow(dead_code)]
    pub fn maybe(
        &self,
        #[allow(
            clippy::needless_pass_by_value,
            reason = "We intentionally do this because `other` should not be used after extension."
        )]
        status: Result<(), RuntimeExtensionErrors>,
    ) {
        if let Err(err) = status {
            self.extend(err);
        }
    }

    /// get a copy of the error at the index if it exists.
    #[allow(dead_code)]
    pub fn get(
        &self,
        #[allow(
            clippy::needless_pass_by_value,
            reason = "We intentionally do this because `other` should not be used after extension."
        )]
        index: usize,
    ) -> Option<RuntimeExtensionError> {
        self.0
            .get(index)
            .map(orx_concurrent_vec::ConcurrentElement::cloned)
    }
}

/// Generic `RuntimeExtensionError` with specific Kinds of errors within it.
#[derive(Clone, thiserror::Error, Debug)]
#[allow(dead_code)]
#[error("{rte_metadata}@({file}:{line}) : {kind}")]
pub(crate) struct RuntimeExtensionError {
    /// Metadata about the runtime extension who's resource which failed.
    pub rte_metadata: RteMetadata,
    /// File where the error occurred.
    pub file: String,
    /// Line in file where we recorded the error.
    pub line: u32,
    /// What kind of error (and its parameters)
    pub kind: RuntimeExtensionErrorKind,
}

/// All individual errors that a runtime extension can make.
///
/// Note: ALL Error types MUST include:
/// * `rte_metadata`, `file`, `line` and `error`.
///
/// All other data is customized to match the error.
#[derive(Clone, thiserror::Error, Debug)]
#[allow(dead_code)]
pub(crate) enum RuntimeExtensionErrorKind {
    /// A Resource Allocation Failed
    #[error("resource: {resource} allocation error. Expected {expected}, Available {available}")]
    #[allow(dead_code)]
    ResourceAllocation {
        /// Resource that actually failed
        resource: String,
        /// How much of the resource we required
        expected: String,
        /// How much of the resource was available.
        available: String,
    },

    /// A Resource Deallocation Failed
    #[error("resource: {resource} deallocation error. {reason}")]
    #[allow(dead_code)]
    ResourceDeallocation {
        /// Resource that actually failed
        resource: String,
        /// Why we failed to deallocate the resource.
        reason: String,
    },

    /// A Permissions Error Occurred
    #[error("permission: {permission} not granted. Requires {requires}, Current {current})")]
    #[allow(dead_code)]
    Permission {
        /// Permission check the runtime extension lacks or failed
        permission: String,
        /// What permission would be required to solve the problem
        requires: String,
        /// The permission we have
        current: String,
    },

    /// A Missing Export has been detected in a Module
    #[error("missing export: {export} not present, required by {required_by}.")]
    #[allow(dead_code)]
    MissingExport {
        /// What is the missing export that the runtime extension expected?
        export: String,
        /// What API Import made the export required.
        required_by: String,
    },

    /// Only use this for errors which should be impossible at runtime, but are
    /// theoretically possible (such a mutex poisoning).
    /// Any other error type should be extrapolated to one of the kinds of errors
    /// here (or a new kind is added) and they should be high level enough that the
    /// handler of the errors can reason through them and provide guidance to the user
    /// about what went wrong, and what they need to do to correct it.
    #[error("impossible: {description}")]
    ImpossibleError {
        /// Permission check the runtime extension lacks or failed
        description: String,
    },
}

/// Adds a `RuntimeExtensionError` to a `RuntimeExtensionErrors` container.
///
/// ONLY used for initialization or finalization, never used for runtime extension
/// methods that wasm would call.
///
/// Automatically sets `rte_metadata`, `file`, `line`,
/// and wraps any error provided in `Arc` internally.
/// `rte_metadata` must exist, but all init and fini functions take
/// it as a parameter, so it will.
///
/// Usage:
/// ```ignore
/// add_rte_error!(errors, ResourceAllocation {
///     resource: "GPU".to_string(),
///     expected: "4".to_string(),
///     available: "2".to_string(),
///     error: io_err, // raw error, not Some(err)
/// });
/// ```
#[macro_export]
macro_rules! add_rte_error {
    // With error provided
    ($container:expr, $rte_metadata:expr, $variant:ident { $($field:ident : $value:expr),* $(,)? } ) => {{
        let err = $crate::runtime_extensions::init::errors::RuntimeExtensionError {
            rte_metadata: $rte_metadata.clone(),
            file: file!().to_string(),
            line: line!(),
            kind: $crate::runtime_extensions::init::errors::RuntimeExtensionErrorKind::$variant {
                $(
                    $field: $value,
                )*
            }
        };
        $container.0.push(err);
    }};
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;
    use crate::runtime_extensions::init::metadata::RteMetadataInner;

    /// Tests that a new `RuntimeExtensionErrors` is empty
    #[test]
    fn test_new_is_empty() {
        let errors = RuntimeExtensionErrors::new();
        assert!(
            errors.is_empty(),
            "A new RuntimeExtensionErrors should be empty"
        );
        assert_eq!(
            errors.len(),
            0,
            "A new RuntimeExtensionErrors should have zero length"
        );
    }

    /// Tests adding a `ResourceAllocation` error via the macro
    #[test]
    fn test_add_resource_allocation_error() {
        let errors = RuntimeExtensionErrors::new();
        let rte_metadata = RteMetadata::new(RteMetadataInner {
            has_constructor: false,
            name: "unit_test",
            path: "unit.test.path",
            file: "unit/test/file",
            module_path: "unit/test/module/path",
            trait_name: "trait::name",
        });

        add_rte_error!(
            errors,
            rte_metadata,
            ResourceAllocation {
                resource: "GPU".to_string(),
                expected: "4".to_string(),
                available: "2".to_string(),
            }
        );

        assert_eq!(errors.len(), 1, "There should be one error collected");

        if let Some(err) = errors.get(0) {
            assert_eq!(
                err.rte_metadata, rte_metadata,
                "RteMetadata should match the one provided"
            );
            assert_eq!(
                err.file,
                file!(),
                "File should match the current file macro"
            ); // Note: this will match the macro invocation line in tests
            assert!(err.line > 0, "Line number should be greater than zero");
            assert!(matches!(
                err.kind,
                RuntimeExtensionErrorKind::ResourceAllocation { .. }
            ));
        } else {
            panic!("Expected a ResourceAllocation variant");
        }
    }

    /// Tests adding multiple errors of different variants
    #[test]
    fn test_add_multiple_error_variants() {
        let errors = RuntimeExtensionErrors::new();
        let rte_metadata = RteMetadata::new(RteMetadataInner {
            has_constructor: false,
            name: "unit_test",
            path: "unit.test.path",
            file: "unit/test/file",
            module_path: "unit/test/module/path",
            trait_name: "trait::name",
        });

        add_rte_error!(
            errors,
            rte_metadata,
            ResourceAllocation {
                resource: "CPU".to_string(),
                expected: "8".to_string(),
                available: "4".to_string(),
            }
        );

        add_rte_error!(
            errors,
            rte_metadata,
            ResourceDeallocation {
                resource: "Memory".to_string(),
                reason: "Memory region is locked and can not be freed.".to_string(),
            }
        );

        add_rte_error!(
            errors,
            rte_metadata,
            Permission {
                permission: "Access".to_string(),
                requires: "Admin".to_string(),
                current: "User".to_string(),
            }
        );

        // Check each variant type
        assert!(matches!(
            errors.get(0).map(|e| e.kind.clone()),
            Some(RuntimeExtensionErrorKind::ResourceAllocation { .. })
        ));
        assert!(matches!(
            errors.get(1).map(|e| e.kind.clone()),
            Some(RuntimeExtensionErrorKind::ResourceDeallocation { .. })
        ));
        assert!(matches!(
            errors.get(2).map(|e| e.kind.clone()),
            Some(RuntimeExtensionErrorKind::Permission { .. })
        ));
        assert!(errors.get(3).is_none());
    }

    /// Tests that `extend()` correctly accumulates errors from another container
    #[test]
    fn test_extend_errors() {
        let errors1 = RuntimeExtensionErrors::new();
        let errors2 = RuntimeExtensionErrors::new();
        let rte_metadata = RteMetadata::new(RteMetadataInner {
            has_constructor: false,
            name: "unit_test",
            path: "unit.test.path",
            file: "unit/test/file",
            module_path: "unit/test/module/path",
            trait_name: "trait::name",
        });

        add_rte_error!(
            errors1,
            rte_metadata,
            ImpossibleError {
                description: "Something impossible failed".to_string(),
            }
        );

        add_rte_error!(
            errors2,
            rte_metadata,
            Permission {
                permission: "Access".to_string(),
                requires: "Admin".to_string(),
                current: "User".to_string(),
            }
        );

        errors1.extend(errors2);

        assert_eq!(
            errors1.len(),
            2,
            "After extend, there should be two errors collected"
        );

        assert!(matches!(
            errors1.get(0).map(|e| e.kind.clone()),
            Some(RuntimeExtensionErrorKind::ImpossibleError { .. })
        ));
        assert!(matches!(
            errors1.get(1).map(|e| e.kind.clone()),
            Some(RuntimeExtensionErrorKind::Permission { .. })
        ));
    }
}
