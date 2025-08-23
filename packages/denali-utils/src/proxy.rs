use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    sync::{Arc, Mutex},
};

use thiserror::Error;

const CLIENT_MIN_ID: u32 = 0x00000001;
const CLIENT_MAX_ID: u32 = 0xfeffffff;

#[derive(Debug, Clone)]
pub struct IdManagerInner {
    next: u32,
    free_list: BinaryHeap<Reverse<u32>>,
}

impl IdManagerInner {
    pub fn new() -> Self {
        Self {
            next: CLIENT_MIN_ID,
            free_list: BinaryHeap::<Reverse<u32>>::new(),
        }
    }

    /// Gets the next available id
    pub fn alloc_id(&mut self) -> Result<u32, ProxyError> {
        if self.next > CLIENT_MAX_ID && self.free_list.is_empty() {
            return Err(ProxyError::IdOutsideOfRange(self.next));
        }

        let id = if let Some(&Reverse(free_id)) = self.free_list.peek()
            && free_id < self.next
        {
            self.free_list.pop();
            free_id
        } else {
            let id = self.next;
            self.next += 1;
            id
        };

        Ok(id)
    }

    /// Return a deleted ID to the pool of available IDs.
    pub fn recycle_id(&mut self, id: u32) {
        if id == self.next - 1 {
            self.next -= 1;

            while let Some(&Reverse(top)) = self.free_list.peek() {
                if top + 1 == self.next {
                    self.free_list.pop();
                    self.next -= 1;
                } else {
                    break;
                }
            }
        } else {
            self.free_list.push(Reverse(id));
        }
    }
}

impl Default for IdManagerInner {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default)]
pub struct IdManager(Arc<Mutex<IdManagerInner>>);
impl IdManager {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(IdManagerInner::new())))
    }
    pub fn alloc_id(&self) -> Result<u32, ProxyError> {
        let mut inner = self.0.lock().unwrap();
        inner.alloc_id()
    }
    pub fn recycle_id(&self, id: u32) {
        let mut inner = self.0.lock().unwrap();
        inner.recycle_id(id);
    }
}

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

    pub fn new(version: u32, shared_manager: IdManager) -> Result<Self, ProxyError> {
        let id = shared_manager.alloc_id().unwrap();
        Ok(Self {
            id,
            version,
            id_manager: shared_manager.clone(),
        })
    }

    pub fn create_object<T: super::Interface + From<Self>>(
        &self,
        version: u32,
    ) -> Result<T, ProxyError> {
        Self::new(version, self.id_manager.clone()).map(From::<Self>::from)
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ProxyError {
    #[error("Proxy ID {0} is above the maximum allowed ID range ({CLIENT_MAX_ID})")]
    IdOutsideOfRange(u32),
}