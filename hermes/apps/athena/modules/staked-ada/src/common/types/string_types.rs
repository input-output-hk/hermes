//! Simple string types.
//!
//! This code comes from Poem, but it is not exported by Poem, so replicated here.
//!
//! Original Source: <https://raw.githubusercontent.com/poem-web/poem/refs/heads/master/poem-openapi/src/types/string_types.rs>

/// Macro to make creating validated and documented string types much easier.
///
/// ## Parameters
///
/// * `$ty` - The Type name to create. Example `MyNewType`.
/// * `$type_name` - The `OpenAPI` name for the type. Almost always going to be `string`.
/// * `$format` - The `OpenAPI` format for the type. Where possible use a defined
///   `OpenAPI` or `JsonSchema` format.
/// * `$schema` - A Poem `MetaSchema` which defines all the schema parameters for the
///   type.
/// * `$validation` - *OPTIONAL* Validation function to apply to the string value.
///
///
/// ## Example
///
/// ```ignore
/// impl_string_types!(MyNewType, "string", "date", MyNewTypeSchema, SomeValidationFunction);
/// ```
///
/// Is the equivalent of:
///
/// ```ignore
/// #[derive(Debug, Clone, Eq, PartialEq, Hash)]
/// pub(crate) struct MyNewType(pub String);
///
/// impl <stuff> for MyNewType { ... }
/// ```
macro_rules! impl_string_types {
    ($(#[$docs:meta])* $ty:ident, $type_name:literal, $format:expr) => {
        impl_string_types!($(#[$docs])* $ty, $type_name, $format, |_| true);
    };

    ($(#[$docs:meta])* $ty:ident, $type_name:literal, $format:expr, $validator:expr) => {
        $(#[$docs])*
        #[derive(Debug, Clone, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
        pub(crate) struct $ty(String);

        impl std::ops::Deref for $ty {
            type Target = String;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $ty {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl AsRef<str> for $ty {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl From<$ty> for String {
            fn from(val: $ty) -> Self {
                val.0
            }
        }
    };
}
pub(crate) use impl_string_types;
