//! Generalized, type safe `wasmtime::component::Resource<T>` manager implementation.

use std::sync::atomic::{AtomicU32, Ordering};

use dashmap::DashMap;

use crate::app::ApplicationName;

/// `ResourceManager` struct.
/// - `WitType` represents the type from the wit file definitions and which will appear in
/// the `wasmtime::component::Resource<WitType>` object.
/// - `RustType` actually the type which is binded to the `WitType` and holds all the data
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
        let mut available_address = self.available_address.load(Ordering::Acquire);
        self.state.insert(available_address, object);

        // Increment the value of the available address to 1,
        // so that it can be used for the next resource.
        // Under the assumption that `ResourceManager` will not handle too many resources at once,
        // and will not hold resources for too long, saturating increment is safe.
        available_address = available_address.saturating_add(1);
        self.available_address
            .store(available_address, Ordering::Release);

        wasmtime::component::Resource::new_own(available_address)
    }

    /// Creates a new owned resource from the given object.
    /// Stores a resources link to the original object in the resource manager.
    pub(crate) fn get_object(
        &self, resource: &wasmtime::component::Resource<WitType>,
    ) -> wasmtime::Result<RustType>
    where RustType: Clone {
        self.state
            .get(&resource.rep())
            .map(|r| r.value().clone())
            .ok_or(wasmtime::Error::msg("Resource not found"))
    }

    /// Removes the resource from the resource manager.
    /// Similar to the `drop` function, resouce is releasing and consumed by this
    /// function, thats why it is passed by value.
    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn delete_resource(&self, resource: wasmtime::component::Resource<WitType>) {
        self.state.remove(&resource.rep());
    }
}

/// `ApplicationResourceManager` struct.
/// - `WitType` represents the type from the wit file definitions and which will appear in
/// the `wasmtime::component::Resource<WitType>` object.
/// - `RustType` actually the type which is binded to the `WitType` and holds all the data
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

    /// Creates a new owned resource from the given object.
    /// Stores a resources link to the original object in the resource manager.
    pub(crate) fn create_resource(
        &self, app_name: ApplicationName, object: RustType,
    ) -> wasmtime::component::Resource<WitType> {
        let app_state = self.state.entry(app_name).or_insert(ResourceManager::new());
        app_state.create_resource(object)
    }

    /// Creates a new owned resource from the given object.
    /// Stores a resources link to the original object in the resource manager.
    pub(crate) fn get_object(
        &self, app_name: ApplicationName, resource: &wasmtime::component::Resource<WitType>,
    ) -> wasmtime::Result<RustType>
    where RustType: Clone {
        let app_state = self.state.entry(app_name).or_insert(ResourceManager::new());
        app_state.get_object(resource)
    }

    /// Removes the resource from the resource manager.
    /// Similar to the `drop` function, resouce is releasing and consumed by this
    /// function, thats why it is passed by value.
    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn delete_resource(
        &self, app_name: ApplicationName, resource: wasmtime::component::Resource<WitType>,
    ) {
        let app_state = self.state.entry(app_name).or_insert(ResourceManager::new());
        app_state.delete_resource(resource);
    }
}
