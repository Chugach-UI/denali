use std::collections::BTreeMap;

use crate::wire::serde::ObjectId;

use crate::Interface;
use crate::proxy::{Proxy, ProxyUpcast, SharedProxyState};

pub trait Store {
    /// Insert a new object into the store.
    fn insert_interface<I: Interface>(&mut self, interface: I, version: u32);
    /// Insert a new object into the store.
    fn insert_proxy(&mut self, interface: String, version: u32, proxy: Proxy);
    /// Take ownership of an object by its ID, if it exists and matches the requested interface and version.
    fn take<I: Interface>(&mut self, id: &ObjectId) -> Option<I>;
    fn remove(&mut self, id: &ObjectId);
    /// Get a reference to an object by its ID, if it exists and matches the requested interface and version.
    fn get<I: Interface + ProxyUpcast>(&self, id: &ObjectId) -> Option<&I>;
    /// Get references to all objects that match the requested interface and version.
    fn get_all<I: Interface + ProxyUpcast>(&self) -> Vec<&I>;
}

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
pub struct InterfaceStore {
    objects: BTreeMap<ObjectId, Object>,
    shared_state: SharedProxyState,
}
impl InterfaceStore {
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
    pub fn insert_proxy(&mut self, interface: String, version: u32, proxy: Proxy) {
        let mut map = self.shared_state.interface_map.lock().unwrap();
        map.insert(proxy.id(), interface.clone());
        self.objects.insert(
            proxy.id(),
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

impl Store for InterfaceStore {
    fn get<I: Interface + ProxyUpcast>(&self, id: &ObjectId) -> Option<&I> {
        self.get(id)
    }

    fn get_all<I: Interface + ProxyUpcast>(&self) -> Vec<&I> {
        self.get_all()
    }

    fn insert_interface<I: Interface>(&mut self, interface: I, version: u32) {
        self.insert_interface(interface, version);
    }

    fn insert_proxy(&mut self, interface: std::string::String, version: u32, proxy: Proxy) {
        self.insert_proxy(interface, version, proxy);
    }

    fn remove(&mut self, id: &ObjectId) {
        self.remove(id);
    }

    fn take<I: Interface>(&mut self, id: &ObjectId) -> Option<I> {
        self.take(id)
    }
}
