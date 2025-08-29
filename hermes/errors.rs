pub(crate) mod errors {
    //! Initialization Finalization Errors.
    //!
    //! Note, while this is similare to `ProblemReport`,
    //! it is strictly an Error encountered during initialization/finalization.
    use std::{fmt::Display, sync::Arc};
    use orx_concurrent_vec::ConcurrentVec;
    use crate::runtime_extensions::traits::{RteMetadata, RteMetadataInner};
    /// All errors we encountered while running initialization or finalization of a runtime extension.
    pub(crate) struct RuntimeExtensionErrors(Arc<ConcurrentVec<RuntimeExtensionError>>);
    #[automatically_derived]
    impl ::core::clone::Clone for RuntimeExtensionErrors {
        #[inline]
        fn clone(&self) -> RuntimeExtensionErrors {
            RuntimeExtensionErrors(::core::clone::Clone::clone(&self.0))
        }
    }
    #[allow(unused_qualifications)]
    #[automatically_derived]
    impl ::thiserror::__private::Error for RuntimeExtensionErrors {}
    #[automatically_derived]
    impl ::core::fmt::Debug for RuntimeExtensionErrors {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(
                f,
                "RuntimeExtensionErrors",
                &&self.0,
            )
        }
    }
    impl Display for RuntimeExtensionErrors {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("")
        }
    }
    impl RuntimeExtensionErrors {
        /// Create a new set of Runtime Extension Errors
        fn new() -> Self {
            RuntimeExtensionErrors(Arc::new(ConcurrentVec::new()))
        }
        /// Are there any errors present?
        fn is_empty(&self) -> bool {
            self.0.is_empty()
        }
        /// Extends the `RuntimeExtensionErrors` with another set of errors.
        /// This is how we accumulate errors from multiple runtime extensions.
        fn extend(&self, other: Self) {
            for error in other.0.iter() {
                self.0.push(error.cloned());
            }
        }
    }
    /// All individual errors that a runtime extension can make.
    pub(crate) enum RuntimeExtensionError {
        /// A Resource Allocation Failed
        #[error(
            "{rte_metadata}: resource: {resource} allocation error. Expected {expected}, Available {available})"
        )]
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
            error: Option<Arc<dyn std::error::Error + Send + Sync>>,
        },
        /// A Resource Deallocation Failed
        #[error(
            "{rte_metadata}: resource: {resource} deallocation error. Expected {expected}, Available {available})"
        )]
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
            error: Option<Arc<dyn std::error::Error + Send + Sync>>,
        },
        /// A Permissions Error Occurred
        #[error(
            "{rte_metadata}: permission: {permission} not granted. Requires {requires}, Current {current})"
        )]
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
            error: Option<Arc<dyn std::error::Error + Send + Sync>>,
        },
        /// A Permissions Error Occurred
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
            error: Option<Arc<dyn std::error::Error + Send + Sync>>,
        },
    }
    #[automatically_derived]
    impl ::core::clone::Clone for RuntimeExtensionError {
        #[inline]
        fn clone(&self) -> RuntimeExtensionError {
            match self {
                RuntimeExtensionError::ResourceAllocation {
                    rte_metadata: __self_0,
                    file: __self_1,
                    line: __self_2,
                    resource: __self_3,
                    expected: __self_4,
                    available: __self_5,
                    error: __self_6,
                } => {
                    RuntimeExtensionError::ResourceAllocation {
                        rte_metadata: ::core::clone::Clone::clone(__self_0),
                        file: ::core::clone::Clone::clone(__self_1),
                        line: ::core::clone::Clone::clone(__self_2),
                        resource: ::core::clone::Clone::clone(__self_3),
                        expected: ::core::clone::Clone::clone(__self_4),
                        available: ::core::clone::Clone::clone(__self_5),
                        error: ::core::clone::Clone::clone(__self_6),
                    }
                }
                RuntimeExtensionError::ResourceDeallocation {
                    rte_metadata: __self_0,
                    file: __self_1,
                    line: __self_2,
                    resource: __self_3,
                    expected: __self_4,
                    available: __self_5,
                    error: __self_6,
                } => {
                    RuntimeExtensionError::ResourceDeallocation {
                        rte_metadata: ::core::clone::Clone::clone(__self_0),
                        file: ::core::clone::Clone::clone(__self_1),
                        line: ::core::clone::Clone::clone(__self_2),
                        resource: ::core::clone::Clone::clone(__self_3),
                        expected: ::core::clone::Clone::clone(__self_4),
                        available: ::core::clone::Clone::clone(__self_5),
                        error: ::core::clone::Clone::clone(__self_6),
                    }
                }
                RuntimeExtensionError::Permission {
                    rte_metadata: __self_0,
                    file: __self_1,
                    line: __self_2,
                    permission: __self_3,
                    requires: __self_4,
                    current: __self_5,
                    error: __self_6,
                } => {
                    RuntimeExtensionError::Permission {
                        rte_metadata: ::core::clone::Clone::clone(__self_0),
                        file: ::core::clone::Clone::clone(__self_1),
                        line: ::core::clone::Clone::clone(__self_2),
                        permission: ::core::clone::Clone::clone(__self_3),
                        requires: ::core::clone::Clone::clone(__self_4),
                        current: ::core::clone::Clone::clone(__self_5),
                        error: ::core::clone::Clone::clone(__self_6),
                    }
                }
                RuntimeExtensionError::Runtime {
                    file: __self_0,
                    line: __self_1,
                    rte_metadata: __self_2,
                    description: __self_3,
                    error: __self_4,
                } => {
                    RuntimeExtensionError::Runtime {
                        file: ::core::clone::Clone::clone(__self_0),
                        line: ::core::clone::Clone::clone(__self_1),
                        rte_metadata: ::core::clone::Clone::clone(__self_2),
                        description: ::core::clone::Clone::clone(__self_3),
                        error: ::core::clone::Clone::clone(__self_4),
                    }
                }
            }
        }
    }
    #[allow(unused_qualifications)]
    #[automatically_derived]
    impl ::thiserror::__private::Error for RuntimeExtensionError {}
    #[allow(unused_qualifications)]
    #[automatically_derived]
    impl ::core::fmt::Display for RuntimeExtensionError {
        fn fmt(&self, __formatter: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            use ::thiserror::__private::AsDisplay as _;
            #[allow(unused_variables, deprecated, clippy::used_underscore_binding)]
            match self {
                RuntimeExtensionError::ResourceAllocation {
                    rte_metadata,
                    file,
                    line,
                    resource,
                    expected,
                    available,
                    error,
                } => {
                    match (
                        rte_metadata.as_display(),
                        resource.as_display(),
                        expected.as_display(),
                        available.as_display(),
                    ) {
                        (
                            __display_rte_metadata,
                            __display_resource,
                            __display_expected,
                            __display_available,
                        ) => {
                            __formatter
                                .write_fmt(
                                    format_args!(
                                        "{0}: resource: {1} allocation error. Expected {2}, Available {3})",
                                        __display_rte_metadata,
                                        __display_resource,
                                        __display_expected,
                                        __display_available,
                                    ),
                                )
                        }
                    }
                }
                RuntimeExtensionError::ResourceDeallocation {
                    rte_metadata,
                    file,
                    line,
                    resource,
                    expected,
                    available,
                    error,
                } => {
                    match (
                        rte_metadata.as_display(),
                        resource.as_display(),
                        expected.as_display(),
                        available.as_display(),
                    ) {
                        (
                            __display_rte_metadata,
                            __display_resource,
                            __display_expected,
                            __display_available,
                        ) => {
                            __formatter
                                .write_fmt(
                                    format_args!(
                                        "{0}: resource: {1} deallocation error. Expected {2}, Available {3})",
                                        __display_rte_metadata,
                                        __display_resource,
                                        __display_expected,
                                        __display_available,
                                    ),
                                )
                        }
                    }
                }
                RuntimeExtensionError::Permission {
                    rte_metadata,
                    file,
                    line,
                    permission,
                    requires,
                    current,
                    error,
                } => {
                    match (
                        rte_metadata.as_display(),
                        permission.as_display(),
                        requires.as_display(),
                        current.as_display(),
                    ) {
                        (
                            __display_rte_metadata,
                            __display_permission,
                            __display_requires,
                            __display_current,
                        ) => {
                            __formatter
                                .write_fmt(
                                    format_args!(
                                        "{0}: permission: {1} not granted. Requires {2}, Current {3})",
                                        __display_rte_metadata,
                                        __display_permission,
                                        __display_requires,
                                        __display_current,
                                    ),
                                )
                        }
                    }
                }
                RuntimeExtensionError::Runtime {
                    file,
                    line,
                    rte_metadata,
                    description,
                    error,
                } => {
                    match (rte_metadata.as_display(), description.as_display()) {
                        (__display_rte_metadata, __display_description) => {
                            __formatter
                                .write_fmt(
                                    format_args!(
                                        "{0}: runtime: {1}",
                                        __display_rte_metadata,
                                        __display_description,
                                    ),
                                )
                        }
                    }
                }
            }
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for RuntimeExtensionError {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                RuntimeExtensionError::ResourceAllocation {
                    rte_metadata: __self_0,
                    file: __self_1,
                    line: __self_2,
                    resource: __self_3,
                    expected: __self_4,
                    available: __self_5,
                    error: __self_6,
                } => {
                    let names: &'static _ = &[
                        "rte_metadata",
                        "file",
                        "line",
                        "resource",
                        "expected",
                        "available",
                        "error",
                    ];
                    let values: &[&dyn ::core::fmt::Debug] = &[
                        __self_0,
                        __self_1,
                        __self_2,
                        __self_3,
                        __self_4,
                        __self_5,
                        &__self_6,
                    ];
                    ::core::fmt::Formatter::debug_struct_fields_finish(
                        f,
                        "ResourceAllocation",
                        names,
                        values,
                    )
                }
                RuntimeExtensionError::ResourceDeallocation {
                    rte_metadata: __self_0,
                    file: __self_1,
                    line: __self_2,
                    resource: __self_3,
                    expected: __self_4,
                    available: __self_5,
                    error: __self_6,
                } => {
                    let names: &'static _ = &[
                        "rte_metadata",
                        "file",
                        "line",
                        "resource",
                        "expected",
                        "available",
                        "error",
                    ];
                    let values: &[&dyn ::core::fmt::Debug] = &[
                        __self_0,
                        __self_1,
                        __self_2,
                        __self_3,
                        __self_4,
                        __self_5,
                        &__self_6,
                    ];
                    ::core::fmt::Formatter::debug_struct_fields_finish(
                        f,
                        "ResourceDeallocation",
                        names,
                        values,
                    )
                }
                RuntimeExtensionError::Permission {
                    rte_metadata: __self_0,
                    file: __self_1,
                    line: __self_2,
                    permission: __self_3,
                    requires: __self_4,
                    current: __self_5,
                    error: __self_6,
                } => {
                    let names: &'static _ = &[
                        "rte_metadata",
                        "file",
                        "line",
                        "permission",
                        "requires",
                        "current",
                        "error",
                    ];
                    let values: &[&dyn ::core::fmt::Debug] = &[
                        __self_0,
                        __self_1,
                        __self_2,
                        __self_3,
                        __self_4,
                        __self_5,
                        &__self_6,
                    ];
                    ::core::fmt::Formatter::debug_struct_fields_finish(
                        f,
                        "Permission",
                        names,
                        values,
                    )
                }
                RuntimeExtensionError::Runtime {
                    file: __self_0,
                    line: __self_1,
                    rte_metadata: __self_2,
                    description: __self_3,
                    error: __self_4,
                } => {
                    ::core::fmt::Formatter::debug_struct_field5_finish(
                        f,
                        "Runtime",
                        "file",
                        __self_0,
                        "line",
                        __self_1,
                        "rte_metadata",
                        __self_2,
                        "description",
                        __self_3,
                        "error",
                        &__self_4,
                    )
                }
            }
        }
    }
    /// Tests adding a ResourceAllocation error via the macro
    pub fn test_add_resource_allocation_error() {
        let errors = RuntimeExtensionErrors::new();
        let rte_metadata = RteMetadata::new(RteMetadataInner {
            has_constructor: false,
            name: "unit_test",
            path: "unit.test.path",
            file: "unit/test/file",
            module_path: "unit/test/module/path",
            trait_name: "trait::name",
        });
        {
            let err = RuntimeExtensionError::ResourceAllocation {
                rte_metadata: rte_metadata.clone(),
                resource: "GPU".to_string(),
                expected: "4".to_string(),
                available: "2".to_string(),
                error: None,
                file: "bin/src/runtime_extensions/traits/errors.rs".to_string(),
                line: 187u32,
            };
            errors.0.push(err);
        };
        match (&errors.0.len(), &1) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    let kind = ::core::panicking::AssertKind::Eq;
                    ::core::panicking::assert_failed(
                        kind,
                        &*left_val,
                        &*right_val,
                        ::core::option::Option::Some(
                            format_args!("There should be one error collected"),
                        ),
                    );
                }
            }
        };
        if let Some(
            RuntimeExtensionError::ResourceAllocation { rte_metadata: m, file, line, .. },
        ) = errors.0.get_cloned(0)
        {
            match (&m, &rte_metadata) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("RteMetadata should match the one provided"),
                            ),
                        );
                    }
                }
            };
            match (&file, &"bin/src/runtime_extensions/traits/errors.rs") {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                        let kind = ::core::panicking::AssertKind::Eq;
                        ::core::panicking::assert_failed(
                            kind,
                            &*left_val,
                            &*right_val,
                            ::core::option::Option::Some(
                                format_args!("File should match the current file macro"),
                            ),
                        );
                    }
                }
            };
            if !(line > 0) {
                {
                    ::core::panicking::panic_fmt(
                        format_args!("Line number should be greater than zero"),
                    );
                }
            }
        } else {
            {
                ::core::panicking::panic_fmt(
                    format_args!("Expected a ResourceAllocation variant"),
                );
            };
        }
    }
}
