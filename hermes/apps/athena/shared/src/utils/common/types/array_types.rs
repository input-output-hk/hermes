//! Simple array types implementor.

/// Macro to make creating validated and documented array types much easier.
///
/// ## Parameters
///
/// * `$ty` - The Type name to create. Example `MyNewType`.
/// * `$type_name` - The `OpenAPI` name for the type. Almost always going to be `string`.
/// * `$item_ty` - The Type name of the item inside this Type.
/// * `$validation` - *OPTIONAL* Validation function to apply to the string value.
macro_rules! impl_array_types {
    ($(#[$docs:meta])* $ty:ident, $item_ty:ident) => {
        impl_array_types!($(#[$docs])* $ty, $item_ty, |_| true);
    };

    ($(#[$docs:meta])* $ty:ident, $item_ty:ident, $validator:expr) => {
        $(#[$docs])*
        #[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
        #[allow(missing_docs)]
        pub struct $ty(Vec<$item_ty>);

        impl From<Vec<$item_ty>> for $ty {
            fn from(value: Vec<$item_ty>) -> Self {
                Self(value)
            }
        }

        impl std::ops::Deref for $ty {
            type Target = Vec<$item_ty>;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $ty {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };
}
pub(crate) use impl_array_types;
