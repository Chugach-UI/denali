use crossbeam::channel::Sender;

use super::id_manager::{IdManager, IdManagerError};

#[derive(Debug, Clone)]
pub struct RequestMessage {
    pub fds: Vec<std::os::unix::io::RawFd>,
    pub buffer: Vec<u8>,
}

pub struct Proxy {
    id: u32,
    version: u32,
    id_manager: IdManager,
    request_sender: Sender<RequestMessage>,
}

impl Proxy {
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn new(
        version: u32,
        shared_manager: IdManager,
        request_sender: Sender<RequestMessage>,
    ) -> Result<Self, IdManagerError> {
        let id = shared_manager.alloc_id()?;
        Ok(Self {
            id,
            version,
            id_manager: shared_manager,
            request_sender,
        })
    }

    pub fn create_object<T: super::Interface>(&self, version: u32) -> Result<T, IdManagerError> {
        Self::new(
            version,
            self.id_manager.clone(),
            self.request_sender.clone(),
        )
        .map(From::from)
    }

    pub fn send_request(&self, request: RequestMessage) {
        self.request_sender.send(request).unwrap();
    }
}
