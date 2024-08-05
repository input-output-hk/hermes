//! Generalized, type safe `wasmtime::component::Resource<T>` manager implementation.

#![allow(dead_code)]

use std::sync::atomic::{AtomicU32, Ordering};

use dashmap::DashMap;

use crate::app::ApplicationName;

/// `ResourceManager` struct.
pub(crate) struct ResourceManager<T> {
    /// Map of id to resource object.
    state: DashMap<u32, T>,
    /// Next available address id of the resource.
    available_address: AtomicU32,
}

impl<T> ResourceManager<T>
where T: 'static
{
    /// Creates new `ResourceManager` instance.
    pub(crate) fn new() -> Self {
        Self {
            state: DashMap::new(),
            available_address: AtomicU32::default(),
        }
    }

    /// Creates a new owned resource from the given object.
    /// Stores a resources link to the original object in the resource manager.
    pub(crate) fn create_resource(&mut self, object: T) -> wasmtime::component::Resource<T> {
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
        &self, resource: &wasmtime::component::Resource<T>,
    ) -> wasmtime::Result<T>
    where T: Clone {
        self.state
            .get(&resource.rep())
            .map(|r| r.value().clone())
            .ok_or(wasmtime::Error::msg("Resource not found"))
    }

    /// Removes the resource from the resource manager.
    /// Similar to the `drop` function, resouce is releasing and consumed by this
    /// function, thats why it is passed by value.
    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn delete_resource(&mut self, resource: wasmtime::component::Resource<T>) {
        self.state.remove(&resource.rep());
    }
}

/// `ApplicationResourceManager` struct.
pub(crate) struct ApplicationResourceManager<T> {
    /// Map of id to resource object.
    state: DashMap<(ApplicationName, u32), T>,
    /// Next available address id of the resource.
    available_address: AtomicU32,
}

impl<T> ApplicationResourceManager<T>
where T: 'static
{
    /// Creates new `ApplicationResourceManager` instance.
    pub(crate) fn new() -> Self {
        Self {
            state: DashMap::new(),
            available_address: AtomicU32::default(),
        }
    }

    /// Creates a new owned resource from the given object.
    /// Stores a resources link to the original object in the resource manager.
    pub(crate) fn create_resource(
        &mut self, app_name: ApplicationName, object: T,
    ) -> wasmtime::component::Resource<T> {
        let mut available_address = self.available_address.load(Ordering::Acquire);
        self.state.insert((app_name, available_address), object);

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
        &self, app_name: ApplicationName, resource: &wasmtime::component::Resource<T>,
    ) -> wasmtime::Result<T>
    where T: Clone {
        self.state
            .get(&(app_name, resource.rep()))
            .map(|r| r.value().clone())
            .ok_or(wasmtime::Error::msg("Resource not found"))
    }

    /// Removes the resource from the resource manager.
    /// Similar to the `drop` function, resouce is releasing and consumed by this
    /// function, thats why it is passed by value.
    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn delete_resource(
        &mut self, app_name: ApplicationName, resource: wasmtime::component::Resource<T>,
    ) {
        self.state.remove(&(app_name, resource.rep()));
    }
}
