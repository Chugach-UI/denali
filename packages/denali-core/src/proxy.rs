use std::{collections::BTreeMap, os::fd::RawFd, rc::Rc, sync::Mutex};

use tokio::sync::mpsc::UnboundedSender;

use crate::{Object, wire::serde::ObjectId};

use super::id_manager::{IdManager, IdManagerError};

#[derive(Debug, Clone)]
pub struct RequestMessage {
    pub fds: Vec<RawFd>,
    pub buffer: Vec<u8>,
}

/// A map of object IDs to their interface names.
pub type InterfaceMap = Rc<Mutex<BTreeMap<ObjectId, String>>>;

/// A proxy object representing a remote object on the Wayland server.
pub struct Proxy {
    id: u32,
    version: u32,
    id_manager: IdManager,
    request_sender: UnboundedSender<RequestMessage>,
    interface_map: InterfaceMap,
}

impl Proxy {
    /// Get the unique ID of this proxy.
    #[must_use]
    pub const fn id(&self) -> u32 {
        self.id
    }

    /// Get the version of this proxy.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Create a new proxy object with a unique ID allocated from the given IdManager.
    ///
    /// # Errors
    ///
    /// This function can error if [IdManager::alloc_id] fails to allocate a new ID.
    pub fn new(
        version: u32,
        shared_manager: IdManager,
        request_sender: UnboundedSender<RequestMessage>,
        interface_map: InterfaceMap,
    ) -> Result<Self, IdManagerError> {
        let id = shared_manager.alloc_id()?;

        Ok(Self {
            id,
            version,
            id_manager: shared_manager,
            request_sender,
            interface_map,
        })
    }

    /// Create a new object of the given interface type.
    ///
    /// # Errors
    ///
    /// This function can error if [IdManager::alloc_id] fails to allocate a new ID.
    pub fn create_object<T: super::Interface>(&self, version: u32) -> Result<T, IdManagerError> {
        self.register_interface(T::INTERFACE);
        Self::new(
            version,
            self.id_manager.clone(),
            self.request_sender.clone(),
            self.interface_map.clone(),
        )
        .map(From::from)
    }
    /// Create a new object with the given interface name.
    ///
    /// # Errors
    ///
    /// This function can error if [IdManager::alloc_id] fails to allocate a new ID.
    pub fn create_object_raw(
        &self,
        interface: &str,
        version: u32,
    ) -> Result<Proxy, IdManagerError> {
        self.register_interface(interface);
        Self::new(
            version,
            self.id_manager.clone(),
            self.request_sender.clone(),
            self.interface_map.clone(),
        )
    }

    pub(crate) fn register_interface(&self, interface: &str) {
        let new_id = self.id_manager.peek_next_id().unwrap();
        let mut map = self.interface_map.lock().unwrap();
        map.insert(new_id, interface.to_string());
    }

    /// Send a request over the wire associated with this proxy.
    pub fn send_request(&self, request: RequestMessage) {
        self.request_sender.send(request).unwrap();
    }
}

impl Object for Proxy {
    fn id(&self) -> u32 {
        self.id
    }
    fn send_request(&self, request: RequestMessage) {
        self.send_request(request);
    }
}
