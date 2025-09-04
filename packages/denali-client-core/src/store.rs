use std::collections::BTreeMap;

use denali_core::wire::serde::ObjectId;

use crate::{
    Interface,
    proxy::{Proxy, ProxyUpcast, SharedProxyState},
};

#[derive(Debug, Clone)]
struct Object {
    version: u32,
    interface: String,
    proxy: Proxy,
}

/// A simple in-memory store for Wayland objects.
///
/// Stores can be created with the DisplayConnection
#[derive(Debug, Clone)]
pub struct Store {
    objects: BTreeMap<ObjectId, Object>,
    shared_state: SharedProxyState,
}
impl Store {
    /// Create a new empty store with the given shared proxy state.
    #[must_use]
    pub const fn new(state: SharedProxyState) -> Self {
        Self {
            objects: BTreeMap::new(),
            shared_state: state,
        }
    }

    /// Insert a new object into the store.
    pub fn insert_interface<I: Interface>(&mut self, interface: I, version: u32) {
        self.objects.insert(
            interface.id(),
            Object {
                version,
                interface: I::INTERFACE.to_owned(),
                proxy: interface.into(),
            },
        );
    }

    /// Insert a new object into the store.
    pub fn insert(&mut self, id: ObjectId, version: u32, interface: String) {
        let mut map = self.shared_state.interface_map.lock().unwrap();
        map.insert(id, interface.clone());

        let proxy = Proxy::with_id(
            version,
            id,
            self.shared_state.id_manager.clone(),
            self.shared_state.request_sender.clone(),
            self.shared_state.interface_map.clone(),
        );

        self.objects.insert(
            id,
            Object {
                version,
                interface,
                proxy,
            },
        );
    }

    /// Remove an object from the store by its ID.
    pub fn remove(&mut self, id: &ObjectId) {
        self.objects.remove(id);
    }
    /// Take ownership of an object by its ID, if it exists and matches the requested interface and version.
    pub fn take<I: Interface>(&mut self, id: &ObjectId) -> Option<I> {
        let obj = self.objects.remove(id)?;

        if obj.interface != I::INTERFACE || obj.version < I::MAX_VERSION {
            self.objects.insert(
                *id,
                Object {
                    version: obj.version,
                    interface: obj.interface,
                    proxy: obj.proxy,
                },
            );
            return None;
        }

        Some(I::from(obj.proxy))
    }

    /// Get a reference to an object by its ID, if it exists and matches the requested interface and version.
    #[must_use]
    pub fn get<I: Interface + ProxyUpcast>(&self, id: &ObjectId) -> Option<&I> {
        let obj = self.objects.get(id)?;

        if obj.interface != I::INTERFACE || obj.version > I::MAX_VERSION {
            return None;
        }

        Some(I::upcast_ref(&obj.proxy))
    }

    /// Get references to all objects that match the requested interface and version.
    #[must_use]
    pub fn get_all<I: Interface + ProxyUpcast>(&self) -> Vec<&I> {
        self.objects
            .values()
            .filter_map(|obj| {
                if obj.interface != I::INTERFACE || obj.version > I::MAX_VERSION {
                    return None;
                }

                Some(I::upcast_ref(&obj.proxy))
            })
            .collect()
    }
}
