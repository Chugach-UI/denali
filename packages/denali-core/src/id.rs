//! A thread-safe manager for allocating and recycling unique client IDs.
//!
//! Incorrect management of IDs will lead to the Wayland server terminating the connection.
//! Therefore, it is important to have a robust ID management system in place.
//! This module provides such a system with the [`IdManager`] struct.
//!
//! [`IdManager`] is thread-safe and can be shared across multiple threads.
//!
//! # Example
//!
//! ```
//! use denali_core::id_manager::IdManager;
//!
//! let id_manager = IdManager::new();
//! let id1 = id_manager.alloc_id().unwrap();
//! let id2 = id_manager.alloc_id().unwrap();
//! assert_ne!(id1, id2);
//! id_manager.recycle_id(id1);
//! let id3 = id_manager.alloc_id().unwrap();
//! assert_eq!(id1, id3); // id1 should be reused
//! ```

use std::borrow::Borrow;
use std::num::NonZeroU32;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Mutex;
use std::{cmp::Reverse, collections::BinaryHeap};

use thiserror::Error;

use crate::Interface;
use crate::wire::serde::RawObjectId;

const CLIENT_MIN_ID: RawObjectId = 0x0000_0001;
const CLIENT_MAX_ID: RawObjectId = 0xfeff_ffff;

/// An owned object ID with a dynamic interface.
///
/// See [`ObjectId`] for an owned object ID with a compile-time-known interface.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DynamicObjectId(NonZeroU32);
impl DynamicObjectId {
    /// Creates a new `ObjectId` from a `RawObjectId`.
    ///
    /// # Panics
    ///
    /// This function will panic if the given `id` is zero.
    ///
    /// # Safety
    ///
    /// The returned `ObjectId` must be a valid Wayland object ID for an object that exists.
    #[must_use]
    pub const unsafe fn new(id: RawObjectId) -> Self {
        Self(NonZeroU32::new(id).expect("ObjectId cannot be zero"))
    }

    /// Returns the underlying `RawObjectId` value of the `ObjectId`.
    #[must_use]
    pub const fn get(&self) -> RawObjectId {
        self.0.get()
    }
}
impl Deref for DynamicObjectId {
    type Target = NonZeroU32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<DynamicObjectId> for RawObjectId {
    fn from(val: DynamicObjectId) -> Self {
        val.0.get()
    }
}

/// An owned object ID with a compile-time-known interface.
///
/// See [`DynamicObjectId`] for an owned object ID with a dynamic interface.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId<I: Interface> {
    id: DynamicObjectId,
    _interface: std::marker::PhantomData<I>,
}

impl<I: Interface> ObjectId<I> {
    /// Creates a new `ObjectId` from a `DynamicObjectId`.
    #[must_use]
    pub const fn new(id: DynamicObjectId) -> Self {
        Self {
            id,
            _interface: std::marker::PhantomData,
        }
    }

    /// Returns the underlying `RawObjectId` value of the `TypedObjectId`.
    #[must_use]
    pub const fn get(&self) -> RawObjectId {
        self.id.get()
    }

    /// Returns a reference to the underlying `ObjectId`.
    #[must_use]
    pub const fn as_object_id(&self) -> &DynamicObjectId {
        &self.id
    }

    /// Returns the underlying `ObjectId`.
    #[must_use]
    pub fn into_inner(self) -> DynamicObjectId {
        self.id
    }

    /// Returns a reference to the underlying `ObjectId`.
    #[must_use]
    pub const fn borrowed<'a>(&'a self) -> BorrowedObjectId<'a, I> {
        BorrowedObjectId {
            id: &self.id,
            _interface: std::marker::PhantomData,
        }
    }
}

impl<I: Interface> Deref for ObjectId<I> {
    type Target = DynamicObjectId;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}
impl<I: Interface> From<ObjectId<I>> for RawObjectId {
    fn from(val: ObjectId<I>) -> Self {
        val.get()
    }
}

/// A borrowed reference to an `ObjectId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BorrowedObjectId<'a, I: Interface> {
    id: &'a DynamicObjectId,
    _interface: std::marker::PhantomData<I>,
}

impl<'a, I: Interface> BorrowedObjectId<'a, I> {}
impl<'a, I: Interface> Deref for BorrowedObjectId<'a, I> {
    type Target = DynamicObjectId;

    fn deref(&self) -> &Self::Target {
        self.id
    }
}

#[derive(Debug, Clone)]
struct IdManagerInner {
    next: RawObjectId,
    free_list: BinaryHeap<Reverse<RawObjectId>>,
}

impl IdManagerInner {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            next: CLIENT_MIN_ID,
            free_list: BinaryHeap::<Reverse<RawObjectId>>::new(),
        }
    }

    /// Peeks at the next available id without allocating it.
    pub fn peek_next_id(&self) -> Result<RawObjectId, IdManagerError> {
        if self.next > CLIENT_MAX_ID && self.free_list.is_empty() {
            return Err(IdManagerError::OutOfClientIds(self.next));
        }

        let id = if let Some(&Reverse(free_id)) = self.free_list.peek()
            && free_id < self.next
        {
            free_id
        } else {
            self.next
        };

        Ok(id)
    }

    /// Gets the next available id
    ///
    /// # Errors
    ///
    /// This function will return an error if all client IDs have been exhausted.
    pub fn alloc_id(&mut self) -> Result<RawObjectId, IdManagerError> {
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
    pub fn recycle_id(&mut self, id: RawObjectId) {
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

/// A thread-safe manager for allocating and recycling unique client IDs.
#[derive(Debug, Clone, Default)]
pub struct IdManager(Arc<Mutex<IdManagerInner>>);
impl IdManager {
    #[must_use]
    /// Creates a new `IdManager`.
    ///
    /// The first ID allocated will be `CLIENT_MIN_ID`.
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(IdManagerInner::new())))
    }

    /// Peeks at the next available id without allocating it.
    ///
    /// # Errors
    ///
    /// This function will return an error if all client IDs have been exhausted.
    pub fn peek_next_id(&self) -> Result<RawObjectId, IdManagerError> {
        let inner = self.0.lock().unwrap();
        inner.peek_next_id()
    }

    /// Gets the next available id
    ///
    /// # Errors
    ///
    /// This function will return an error if all client IDs have been exhausted.
    pub fn alloc_id(&self) -> Result<RawObjectId, IdManagerError> {
        let mut inner = self.0.lock().unwrap();
        inner.alloc_id()
    }
    /// Return a deleted ID to the pool of available IDs.
    pub fn recycle_id(&self, id: impl Into<RawObjectId>) {
        let mut inner = self.0.lock().unwrap();
        inner.recycle_id(id.into());
    }
}

/// An error that may occur when allocating a new client ID.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum IdManagerError {
    /// All client IDs have been exhausted
    #[error(
        "All client IDs have been exhausted (ID {0} is out of the range of {CLIENT_MIN_ID} - {CLIENT_MAX_ID})"
    )]
    OutOfClientIds(RawObjectId),
}
