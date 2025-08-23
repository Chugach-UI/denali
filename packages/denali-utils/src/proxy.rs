use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    sync::{Arc, Mutex},
};

const CLIENT_MIN_ID: u32 = 0x00000001;
const CLIENT_MAX_ID: u32 = 0xfeffffff;

pub struct IdManager {
    next: u32,
    free_list: BinaryHeap<Reverse<u32>>,
}

impl IdManager {
    pub fn new() -> Self {
        Self {
            next: CLIENT_MIN_ID,
            free_list: BinaryHeap::<Reverse<u32>>::new(),
        }
    }

    pub fn alloc_id(&mut self) -> Result<u32, String> {
        if self.next > CLIENT_MAX_ID && self.free_list.is_empty() {
            return Err("Client maximum id (0xfeffffff) exceeded".to_string());
        }

        let id = match self.free_list.peek() {
            Some(&Reverse(free_id)) if free_id < self.next => {
                self.free_list.pop();
                free_id
            }
            _ => {
                let id = self.next;
                self.next += 1;
                id
            }
        };

        Ok(id)
    }

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

pub type SharedIdManager = Arc<Mutex<IdManager>>;

pub struct Proxy {
    id: u32,
    version: u32,
    id_manager: SharedIdManager,
}

impl Proxy {
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn new_with_manager(version: u32, shared_manager: SharedIdManager) -> Result<Self, String> {
        let mut manager = shared_manager.lock().unwrap();
        let id = manager.alloc_id().unwrap();
        Ok(Self {
            id,
            version,
            id_manager: shared_manager.clone(),
        })
    }

    pub fn create_object<T: super::Interface + From<Self>>(
        &self,
        version: u32,
    ) -> Result<T, String> {
        Self::new_with_manager(version, self.id_manager.clone()).map(From::<Self>::from)
    }
}
