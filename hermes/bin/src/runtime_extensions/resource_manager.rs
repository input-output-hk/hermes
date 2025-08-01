//! Generalized, type safe `wasmtime::component::Resource<T>` manager implementation.

use std::{
    any::type_name,
    ops::DerefMut,
    sync::atomic::{AtomicU32, Ordering},
};

use dashmap::DashMap;

use crate::app::ApplicationName;

/// `ResourceStorage` struct.
/// - `WitType` represents the type from the wit file definitions and which will appear in
///   the `wasmtime::component::Resource<WitType>` object.
/// - `RustType` actually the type which is bound to the `WitType` and holds all the data
///   needed for the `WitType`.
pub(crate) struct ResourceStorage<WitType, RustType> {
    /// Map of id to resource object.
    state: DashMap<u32, RustType>,
    /// Next available address id of the resource.
    available_address: AtomicU32,
    /// `WitType` type phantom.
    _phantom: std::marker::PhantomData<WitType>,
}

impl<WitType, RustType> ResourceStorage<WitType, RustType>
where WitType: 'static
{
    /// Creates new `ResourceStorage` instance.
    pub(crate) fn new() -> Self {
        Self {
            state: DashMap::new(),
            available_address: AtomicU32::default(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Creates a new owned resource from the given object.
    /// Stores a resources link to the original object in the resource manager.
    pub(crate) fn create_resource(
        &self, object: RustType,
    ) -> wasmtime::component::Resource<WitType> {
        let available_address = self.available_address.load(Ordering::Acquire);
        self.state.insert(available_address, object);

        // Increment the value of the available address to 1,
        // so that it can be used for the next resource.
        // Under the assumption that `ResourceStorage` will not handle too many resources at once,
        // and will not hold resources for too long, saturating increment is safe.
        self.available_address
            .store(available_address.saturating_add(1), Ordering::Release);

        wasmtime::component::Resource::new_own(available_address)
    }

    /// Creates a new owned resource from the given object.
    /// Stores a resources link to the original object in the resource manager.
    ///
    /// NOTE: `&mut self` is used to enable the `rustc` borrow checker and prevent
    /// potential deadlocking of the `DashMap`.
    /// As `ResourceStorage` not supposed to be used in any other cases rather than as a
    /// field of `ApplicationResourceStorage` (so no any `static` variable of this
    /// type will be created), it is fine to add `&mut self` for this method.
    pub(crate) fn get_object<'a>(
        &'a mut self, resource: &wasmtime::component::Resource<WitType>,
    ) -> wasmtime::Result<impl DerefMut<Target = RustType> + 'a> {
        self.state
            .get_mut(&resource.rep())
            .ok_or(Self::resource_not_found_err())
    }

    /// Removes the resource from the resource manager.
    /// Similar to the `drop` function, resource is releasing and consumed by this
    /// function, thats why it is passed by value.
    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn delete_resource(
        &self, resource: wasmtime::component::Resource<WitType>,
    ) -> anyhow::Result<RustType> {
        self.delete_resource_rep(resource.rep())
    }

    /// Removes the resource from the resource manager by its representation.
    /// The resource is properly removed from internal storage.
    /// Returns an error if the resource was not found.
    pub(crate) fn delete_resource_rep(&self, rep: u32) -> anyhow::Result<RustType> {
        self.state
            .remove(&rep)
            .map(|(_, v)| v)
            .ok_or(Self::resource_not_found_err())
    }

    /// Resource not found error message.
    fn resource_not_found_err() -> wasmtime::Error {
        let msg = format!(
            "Resource <{}, {}> not found, need to add resource first by calling `add_resource`",
            type_name::<WitType>(),
            type_name::<RustType>()
        );
        wasmtime::Error::msg(msg)
    }
}

/// `ApplicationResourceStorage` struct.
/// - `WitType` represents the type from the wit file definitions and which will appear in
///   the `wasmtime::component::Resource<WitType>` object.
/// - `RustType` actually the type which is bound to the `WitType` and holds all the data
///   needed for the `WitType`.
pub(crate) struct ApplicationResourceStorage<WitType, RustType> {
    /// Map of app name to resources.
    state: DashMap<ApplicationName, ResourceStorage<WitType, RustType>>,
}

impl<WitType, RustType> ApplicationResourceStorage<WitType, RustType>
where WitType: 'static
{
    /// Creates new `ApplicationResourceStorage` instance.
    pub(crate) fn new() -> Self {
        Self {
            state: DashMap::new(),
        }
    }

    /// Adds new application to the resource manager.
    /// If the application state already exists, do nothing.
    pub(crate) fn add_app(&self, app_name: ApplicationName) {
        if !self.state.contains_key(&app_name) {
            self.state.insert(app_name, ResourceStorage::new());
        }
    }

    /// Get application state from the resource manager.
    /// To increase performance and reduce locking time, it's better to call
    /// `drop(app_state)` immediately when the `app_state` is not needed anymore and don't
    /// wait until it will be released by the compiler.
    ///
    /// **Locking behavior:** May deadlock if called when holding any sort of reference
    /// into the map.
    pub(crate) fn get_app_state<'a>(
        &'a self, app_name: &ApplicationName,
    ) -> anyhow::Result<impl DerefMut<Target = ResourceStorage<WitType, RustType>> + 'a> {
        self.state
            .get_mut(app_name)
            .ok_or_else(|| anyhow::anyhow!(Self::app_not_found_err()))
    }

    /// Removes application and all associated resources from the resource manager.
    #[allow(dead_code)]
    pub(crate) fn remove_app(&self, app_name: &ApplicationName) {
        self.state.remove(app_name);
    }

    /// Application not found error message.
    fn app_not_found_err() -> wasmtime::Error {
        let msg = format!(
        "Application not found for resource <{}, {}>, need to add application first by calling `add_app`",
        type_name::<WitType>(),
        type_name::<RustType>()
    );
        wasmtime::Error::msg(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct WitType;

    #[test]
    fn test_resource_storage() {
        let mut resource_manager = ResourceStorage::<WitType, u32>::new();

        let object = 100;
        let resource = resource_manager.create_resource(object);
        let copied_resource = wasmtime::component::Resource::new_borrow(resource.rep());

        assert_eq!(*resource_manager.get_object(&resource).unwrap(), object);
        assert!(resource_manager.delete_resource(resource).is_ok());
        assert!(resource_manager.get_object(&copied_resource).is_err());
        assert!(resource_manager.delete_resource(copied_resource).is_err());
    }

    #[test]
    fn test_app_resource_storage() {
        let resource_manager = ApplicationResourceStorage::<WitType, u32>::new();
        let app_name_1 = ApplicationName("app_1".to_string());

        {
            assert!(resource_manager.get_app_state(&app_name_1).is_err());
            resource_manager.add_app(app_name_1.clone());
            assert!(resource_manager.get_app_state(&app_name_1).is_ok());
            resource_manager.remove_app(&app_name_1);
            assert!(resource_manager.get_app_state(&app_name_1).is_err());
        }

        {
            // Check preserving app state when instantiating the same app twice
            let object = 100;
            resource_manager.add_app(app_name_1.clone());
            let app_state = resource_manager.get_app_state(&app_name_1).unwrap();
            let res = app_state.create_resource(object);

            drop(app_state);
            resource_manager.add_app(app_name_1.clone());
            let mut app_state = resource_manager.get_app_state(&app_name_1).unwrap();
            assert!(app_state.get_object(&res).is_ok());
        }
    }
}
