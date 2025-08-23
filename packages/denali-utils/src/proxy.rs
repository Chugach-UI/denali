use super::id_manager::{IdManager, IdManagerError};

pub struct Proxy {
    id: u32,
    version: u32,
    id_manager: IdManager,
}

impl Proxy {
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn new(version: u32, shared_manager: IdManager) -> Result<Self, IdManagerError> {
        let id = shared_manager.alloc_id()?;
        Ok(Self {
            id,
            version,
            id_manager: shared_manager.clone(),
        })
    }

    pub fn create_object<T: super::Interface>(&self, version: u32) -> Result<T, IdManagerError> {
        Self::new(version, self.id_manager.clone()).map(From::from)
    }
}
