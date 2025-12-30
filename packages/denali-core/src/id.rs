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
use std::borrow::Cow;
use std::num::NonZeroU32;
use std::ops::Deref;
use std::{cmp::Reverse, collections::BinaryHeap};

use thiserror::Error;

use crate::Interface;
use crate::wire::serde::RawObjectId;

const CLIENT_MIN_ID: RawObjectId = 0x0000_0001;
const CLIENT_MAX_ID: RawObjectId = 0xfeff_ffff;

/// An owned object ID with an unknown interface.
///
/// See [`ObjectId`] for an owned object ID with a compile-time-known interface.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AnyObjectId(NonZeroU32);
impl AnyObjectId {
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
impl Deref for AnyObjectId {
    type Target = NonZeroU32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<AnyObjectId> for RawObjectId {
    fn from(val: AnyObjectId) -> Self {
        val.0.get()
    }
}

/// An object ID tagged with an interface at runtime.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DynamicObjectId<'a>(NonZeroU32, u32, Cow<'a, str>);

impl<'a> DynamicObjectId<'a> {
    /// Creates a new `DynamicObjectId` from a `RawObjectId` and a string.
    #[must_use]
    pub const fn new(id: RawObjectId, version: u32, interface: Cow<'a, str>) -> Self {
        let id = NonZeroU32::new(id).expect("DynamicObjectId cannot be zero");

        Self(id, version, interface)
    }

    /// Returns the underlying `RawObjectId` value of the `ObjectId`.
    #[must_use]
    pub const fn get(&self) -> RawObjectId {
        self.0.get()
    }

    /// Returns the interface
    #[must_use]
    pub fn interface(&self) -> &str {
        self.2.borrow()
    }

    #[must_use]
    pub const fn version(&self) -> u32 {
        self.1
    }
}

#[derive(Error, Debug)]
pub enum DynamicObjectIdError {
    #[error("Interface name does not match")]
    InvalidInterface,
}

impl<'a, I: Interface> TryFrom<DynamicObjectId<'a>> for ObjectId<I> {
    type Error = DynamicObjectIdError;

    fn try_from(value: DynamicObjectId<'a>) -> Result<Self, Self::Error> {
        if value.2 == I::INTERFACE && value.1 <= I::MAX_VERSION {
            Ok(unsafe { ObjectId::from_raw(value.get()) })
        } else {
            Err(DynamicObjectIdError::InvalidInterface)
        }
    }
}

/// An owned object ID with a compile-time-known interface.
///
/// See [`DynamicObjectId`] for an owned object ID with a dynamic interface.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId<I: Interface> {
    id: AnyObjectId,
    _interface: std::marker::PhantomData<I>,
}

impl<I: Interface> ObjectId<I> {
    /// Creates a new `ObjectId` from an `AnyObjectId`.
    #[must_use]
    pub const unsafe fn new(id: AnyObjectId) -> Self {
        Self {
            id,
            _interface: std::marker::PhantomData,
        }
    }

    /// Creates a new `ObjectId` from a `RawObjectId`.
    #[must_use]
    pub const unsafe fn from_raw(id: RawObjectId) -> Self {
        Self {
            id: unsafe { AnyObjectId::new(id) },
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
    pub const fn as_object_id(&self) -> &AnyObjectId {
        &self.id
    }

    /// Returns the underlying `ObjectId`.
    #[must_use]
    pub const fn into_inner(self) -> AnyObjectId {
        self.id
    }
}

impl<I: Interface> Deref for ObjectId<I> {
    type Target = AnyObjectId;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}
impl<I: Interface> From<ObjectId<I>> for RawObjectId {
    fn from(val: ObjectId<I>) -> Self {
        val.get()
    }
}

#[derive(Debug, Clone)]
pub struct IdManager {
    next: RawObjectId,
    free_list: BinaryHeap<Reverse<RawObjectId>>,
}

impl IdManager {
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

    pub unsafe fn alloc_any_id(&mut self) -> Result<AnyObjectId, IdManagerError> {
        self.alloc_id().map(|id| unsafe { AnyObjectId::new(id) })
    }

    pub unsafe fn alloc_typed_id<I: Interface>(&mut self) -> Result<ObjectId<I>, IdManagerError> {
        self.alloc_id().map(|id| unsafe { ObjectId::from_raw(id) })
    }
}
impl Default for IdManager {
    fn default() -> Self {
        Self::new()
    }
}

/// A reference to a [`IdManager`] that only allows for ID allocation
pub struct IdFactory<'a>(&'a mut IdManager);
impl<'a> IdFactory<'a> {
    pub const fn new(manager: &'a mut IdManager) -> Self {
        Self(manager)
    }

    /// Peeks at the next available id without allocating it.
    pub fn peek_next_id(&self) -> Result<RawObjectId, IdManagerError> {
        self.0.peek_next_id()
    }

    /// Gets the next available id
    ///
    /// # Errors
    ///
    /// This function will return an error if all client IDs have been exhausted.
    pub fn alloc_id(&mut self) -> Result<RawObjectId, IdManagerError> {
        self.0.alloc_id()
    }

    pub unsafe fn alloc_any_id(&mut self) -> Result<AnyObjectId, IdManagerError> {
        unsafe { self.0.alloc_any_id() }
    }

    pub unsafe fn alloc_typed_id<I: Interface>(&mut self) -> Result<ObjectId<I>, IdManagerError> {
        unsafe { self.0.alloc_typed_id() }
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
