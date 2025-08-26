use tokio::sync::mpsc::UnboundedSender;

use super::id_manager::{IdManager, IdManagerError};

#[derive(Debug, Clone)]
pub struct RequestMessage {
    pub fds: Vec<std::os::unix::io::RawFd>,
    pub buffer: Vec<u8>,
}

/// A proxy object representing a remote object on the Wayland server.
pub struct Proxy {
    id: u32,
    version: u32,
    id_manager: IdManager,
    request_sender: UnboundedSender<RequestMessage>,
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
    ) -> Result<Self, IdManagerError> {
        let id = shared_manager.alloc_id()?;
        Ok(Self {
            id,
            version,
            id_manager: shared_manager,
            request_sender,
        })
    }

    /// Create a new object of the given interface type.
    ///
    /// # Errors
    /// 
    /// This function can error if [IdManager::alloc_id] fails to allocate a new ID.
    pub fn create_object<T: super::Interface>(&self, version: u32) -> Result<T, IdManagerError> {
        Self::new(
            version,
            self.id_manager.clone(),
            self.request_sender.clone(),
        )
        .map(From::from)
    }

    /// Send a request over the wire associated with this proxy.
    pub fn send_request(&self, request: RequestMessage) {
        self.request_sender.send(request).unwrap();
    }
}
