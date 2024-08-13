//! Generalized, type safe `wasmtime::component::Resource<T>` manager implementation.

use std::{
    any::type_name,
    ops::DerefMut,
    sync::atomic::{AtomicU32, Ordering},
};

use dashmap::DashMap;

use crate::app::ApplicationName;

/// `ResourceManager` struct.
/// - `WitType` represents the type from the wit file definitions and which will appear in
/// the `wasmtime::component::Resource<WitType>` object.
/// - `RustType` actually the type which is bound to the `WitType` and holds all the data
///   needed for the `WitType`.
pub(crate) struct ResourceManager<WitType, RustType> {
    /// Map of id to resource object.
    state: DashMap<u32, RustType>,
    /// Next available address id of the resource.
    available_address: AtomicU32,
    /// `WitType` type phantom.
    _phantom: std::marker::PhantomData<WitType>,
}

impl<WitType, RustType> ResourceManager<WitType, RustType>
where WitType: 'static
{
    /// Creates new `ResourceManager` instance.
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
        // Under the assumption that `ResourceManager` will not handle too many resources at once,
        // and will not hold resources for too long, saturating increment is safe.
        self.available_address
            .store(available_address.saturating_add(1), Ordering::Release);

        wasmtime::component::Resource::new_own(available_address)
    }

    /// Creates a new owned resource from the given object.
    /// Stores a resources link to the original object in the resource manager.
    pub(crate) fn get_object<'a>(
        &'a self, resource: &wasmtime::component::Resource<WitType>,
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
        self.state
            .remove(&resource.rep())
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

/// `ApplicationResourceManager` struct.
/// - `WitType` represents the type from the wit file definitions and which will appear in
/// the `wasmtime::component::Resource<WitType>` object.
/// - `RustType` actually the type which is bound to the `WitType` and holds all the data
///   needed for the `WitType`.
pub(crate) struct ApplicationResourceManager<WitType, RustType> {
    /// Map of app name to resources.
    state: DashMap<ApplicationName, ResourceManager<WitType, RustType>>,
}

impl<WitType, RustType> ApplicationResourceManager<WitType, RustType>
where WitType: 'static
{
    /// Creates new `ApplicationResourceManager` instance.
    pub(crate) fn new() -> Self {
        Self {
            state: DashMap::new(),
        }
    }

    /// Adds new application to the resource manager.
    pub(crate) fn add_app(&self, app_name: ApplicationName) {
        self.state.insert(app_name, ResourceManager::new());
    }

    /// Get application state from the resource manager.
    pub(crate) fn get_app_state<'a>(
        &'a self, app_name: &ApplicationName,
    ) -> anyhow::Result<impl DerefMut<Target = ResourceManager<WitType, RustType>> + 'a> {
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
    #[allow(clippy::unwrap_used)]
    fn test_resource_manager() {
        let resource_manager = ResourceManager::<WitType, u32>::new();

        let object = 100;
        let resource = resource_manager.create_resource(object);
        let copied_resource = wasmtime::component::Resource::new_borrow(resource.rep());

        assert_eq!(*resource_manager.get_object(&resource).unwrap(), object);
        assert!(resource_manager.delete_resource(resource).is_ok());
        assert!(resource_manager.get_object(&copied_resource).is_err());
        assert!(resource_manager.delete_resource(copied_resource).is_err());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_app_resource_manager() {
        let resource_manager = ApplicationResourceManager::<WitType, u32>::new();
        let app_name_1 = ApplicationName("app_1".to_string());

        assert!(resource_manager.get_app_state(&app_name_1).is_err());
        resource_manager.add_app(app_name_1.clone());
        assert!(resource_manager.get_app_state(&app_name_1).is_ok());
        resource_manager.remove_app(&app_name_1);
        assert!(resource_manager.get_app_state(&app_name_1).is_err());
    }
}
