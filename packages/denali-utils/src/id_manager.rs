use std::sync::Arc;
use std::sync::Mutex;
use std::{cmp::Reverse, collections::BinaryHeap};

use thiserror::Error;

const CLIENT_MIN_ID: u32 = 0x00000001;
const CLIENT_MAX_ID: u32 = 0xfeffffff;

#[derive(Debug, Clone)]
struct IdManagerInner {
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
    pub fn alloc_id(&mut self) -> Result<u32, IdManagerError> {
        if self.next > CLIENT_MAX_ID && self.free_list.is_empty() {
            return Err(IdManagerError::OutOfClientIds(self.next));
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
    pub fn alloc_id(&self) -> Result<u32, IdManagerError> {
        let mut inner = self.0.lock().unwrap();
        inner.alloc_id()
    }
    pub fn recycle_id(&self, id: u32) {
        let mut inner = self.0.lock().unwrap();
        inner.recycle_id(id);
    }
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum IdManagerError {
    #[error(
        "All client IDs have been exhausted (ID {0} is out of the range of {CLIENT_MIN_ID} - {CLIENT_MAX_ID})"
    )]
    OutOfClientIds(u32),
}
