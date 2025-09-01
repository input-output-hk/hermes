//! Initialization Finalization Errors.
//!
//! Note, while this is similar to `ProblemReport`,
//! it is strictly an Error encountered during initialization/finalization.
use std::{error::Error, fmt::Display, sync::Arc};

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
        f.write_str("")
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

    /// Push a new `RuntimeExtensionError` into the collection oif errors.
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

    /// Push a new `RuntimeExtensionError` into the collection oif errors.
    /// But ONLY if the status is an error..
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

/// Thread Safe Error Type
type ThreadSafeError = Arc<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Clone, thiserror::Error, Debug)]
#[error("{0}")]
/// Simple Error string wrapper we use when we can't embed the real error in a
/// `RuntimeExtensionError`
pub(crate) struct SimpleError(String);

impl SimpleError {
    /// Create a new `SimpleError` from any standard error
    pub fn new<E: Error>(err: E) -> Self {
        SimpleError(format!("{err}"))
    }
}

impl From<&str> for SimpleError {
    fn from(value: &str) -> Self {
        SimpleError(value.to_string())
    }
}

impl From<String> for SimpleError {
    fn from(value: String) -> Self {
        SimpleError(value)
    }
}

/// All individual errors that a runtime extension can make.
///
/// Note: ALL Error types MUST include:
/// * `rte_metadata`, `file`, `line` and `error`.
///
/// All other data is customized to match the error.
#[derive(Clone, thiserror::Error, Debug)]
#[allow(dead_code)]
pub(crate) enum RuntimeExtensionError {
    /// A Resource Allocation Failed
    #[error("{rte_metadata}: resource: {resource} allocation error. Expected {expected}, Available {available})")]
    #[allow(dead_code)]
    ResourceAllocation {
        /// Metadata about the runtime extension who's resource which failed.
        rte_metadata: RteMetadata,
        /// File where the error occurred.
        file: String,
        /// Line in file where we recorded the error.
        line: u32,
        /// Resource that actually failed
        resource: String,
        /// How much of the resource we required
        expected: String,
        /// How much of the resource was available.
        available: String,
        /// Resource inner error if any
        error: Option<ThreadSafeError>,
    },

    /// A Resource Deallocation Failed
    #[error("{rte_metadata}: resource: {resource} deallocation error. Expected {expected}, Available {available})")]
    #[allow(dead_code)]
    ResourceDeallocation {
        /// Metadata about the runtime extension who's resource which failed.
        rte_metadata: RteMetadata,
        /// File where the error occurred.
        file: String,
        /// Line in file where we recorded the error.
        line: u32,
        /// Resource that actually failed
        resource: String,
        /// How much of the resource we required
        expected: String,
        /// How much of the resource was available.
        available: String,
        /// Resource inner error if any
        error: Option<ThreadSafeError>,
    },

    /// A Permissions Error Occurred
    #[error("{rte_metadata}: permission: {permission} not granted. Requires {requires}, Current {current})")]
    #[allow(dead_code)]
    Permission {
        /// Metadata about the runtime extension who's resource which failed.
        rte_metadata: RteMetadata,
        /// File where the error occurred.
        file: String,
        /// Line in file where we recorded the error.
        line: u32,
        /// Permission check the runtime extension lacks or failed
        permission: String,
        /// What permission would be required to solve the problem
        requires: String,
        /// The permission we have
        current: String,
        /// Resource inner error if any
        error: Option<ThreadSafeError>,
    },

    /// A Missing Export has been detected in a Module
    #[error("{rte_metadata}: missing export: {export} not present, required by {required_by}.")]
    #[allow(dead_code)]
    MissingExport {
        /// Metadata about the runtime extension who's resource which failed.
        rte_metadata: RteMetadata,
        /// File where the error occurred.
        file: String,
        /// Line in file where we recorded the error.
        line: u32,
        /// What event function is missing in a WASM Module
        export: String,
        /// What API Import made the export required.
        required_by: String,
        /// Resource inner error if any
        error: Option<ThreadSafeError>,
    },

    /// A General Rust Runtime Error Occurred (that is not one of the above)
    #[error("{rte_metadata}: runtime: {description}")]
    Runtime {
        /// File where the error occurred.
        file: String,
        /// Line in file where we recorded the error.
        line: u32,
        /// Metadata about the runtime extension who's resource which failed.
        rte_metadata: RteMetadata,
        /// Permission check the runtime extension lacks or failed
        description: String,
        /// Resource inner error if any
        error: Option<ThreadSafeError>,
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
    ($container:expr, $rte_metadata:expr, $variant:ident { $($field:ident : $value:expr),* $(,)? } , $error:expr $(,)? ) => {{
        let err = RuntimeExtensionError::$variant {
            rte_metadata: $rte_metadata.clone(),
            $(
                $field: $value,
            )*
            error: Some(std::sync::Arc::new(Box::new($error))),
            file: file!().to_string(),
            line: line!(),
        };
        $container.0.push(err);
    }};

    // Without error field
    ($container:expr, $rte_metadata:expr, $variant:ident { $($field:ident : $value:expr),* $(,)? }) => {{
        let err = RuntimeExtensionError::$variant {
            rte_metadata: $rte_metadata.clone(),
            $(
                $field: $value,
            )*
            error: None,
            file: file!().to_string(),
            line: line!(),
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

        add_rte_error!(errors, rte_metadata, ResourceAllocation {
            resource: "GPU".to_string(),
            expected: "4".to_string(),
            available: "2".to_string(),
        });

        assert_eq!(errors.len(), 1, "There should be one error collected");

        if let Some(RuntimeExtensionError::ResourceAllocation {
            rte_metadata: m,
            file,
            line,
            ..
        }) = errors.get(0)
        {
            assert_eq!(m, rte_metadata, "RteMetadata should match the one provided");
            assert_eq!(file, file!(), "File should match the current file macro"); // Note: this will match the macro invocation line in tests
            assert!(line > 0, "Line number should be greater than zero");
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

        add_rte_error!(errors, rte_metadata, ResourceAllocation {
            resource: "CPU".to_string(),
            expected: "8".to_string(),
            available: "4".to_string(),
        });

        add_rte_error!(
            errors,
            rte_metadata,
            ResourceDeallocation {
                resource: "Memory".to_string(),
                expected: "16GB".to_string(),
                available: "8GB".to_string(),
            },
            std::io::Error::other("inner error"),
        );

        add_rte_error!(errors, rte_metadata, Permission {
            permission: "Access".to_string(),
            requires: "Admin".to_string(),
            current: "User".to_string(),
        });

        // Check each variant type
        assert!(matches!(
            errors.get(0),
            Some(RuntimeExtensionError::ResourceAllocation { .. })
        ));
        assert!(matches!(
            errors.get(1),
            Some(RuntimeExtensionError::ResourceDeallocation { .. })
        ));
        assert!(matches!(
            errors.get(2),
            Some(RuntimeExtensionError::Permission { .. })
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

        add_rte_error!(errors1, rte_metadata, Runtime {
            description: "Something failed".to_string(),
        });

        add_rte_error!(errors2, rte_metadata, Permission {
            permission: "Access".to_string(),
            requires: "Admin".to_string(),
            current: "User".to_string(),
        });

        errors1.extend(errors2);

        assert_eq!(
            errors1.len(),
            2,
            "After extend, there should be two errors collected"
        );

        assert!(matches!(
            errors1.get(0),
            Some(RuntimeExtensionError::Runtime { .. })
        ));
        assert!(matches!(
            errors1.get(1),
            Some(RuntimeExtensionError::Permission { .. })
        ));
        assert!(matches!(
            errors1.get(2),
            Some(RuntimeExtensionError::Permission { .. })
        ));
    }
}
