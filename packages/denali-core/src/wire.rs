//! Serialization and deserialization of Wayland wire protocol messages.
use std::borrow::Cow;

use thiserror::Error;

#[must_use]
const fn pad_to_32_bits(pos: usize) -> usize {
    (pos + 3) & !3
}

/// Trait for calculating the size of a message.
#[const_trait]
pub trait MessageSize: Sized {
    /// Returns the size of a type as it will be serialized.
    fn size(&self) -> usize {
        size_of::<Self>()
    }
}

/// Trait for encoding a type into a byte slice, as a Wayland message.
pub trait Encode: MessageSize {
    /// Encodes a type into a byte slice.
    fn encode(&self, data: &mut [u8]) -> Result<usize, SerdeError>;
}

/// Trait for decoding a type from a byte slice, as a Wayland message.
pub trait Decode: MessageSize {
    /// Decodes a type from a byte slice.
    fn decode(data: &[u8]) -> Result<Self, SerdeError>;
}

/// A Wayland message header, which prefixes all message sent over the wire.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageHeader {
    /// The object id of the receiving object.
    object_id: u32,
    /// The opcode of the message.
    opcode: u16,
    /// The size of the message in bytes, including the header.
    size: u16,
}

impl const MessageSize for u32 {}
impl const MessageSize for i32 {}

/// A 32 bit object id.
pub type ObjectId = u32;

/// The id of a new object to be created.
pub type NewId = ObjectId;

/// The information necessary to create an object without a specified type.
pub struct GenericNewId<'a> {
    /// The interface (type) of the new object.
    interface: String<'a>,
    /// Version of the interface to be constructed.
    version: u32,
    /// The id of the new object.
    id: NewId,
}

impl<'a> MessageSize for GenericNewId<'_> {
    fn size(&self) -> usize {
        self.interface.size() + self.version.size() + self.id.size()
    }
}

/// A GenericNewId with a compile-time known interface to aid with serialization.
// This will be what is used for 99% of use cases.
pub struct ComptimeGenericNewId {
    /// The interface (type) of the new object.
    interface: ComptimeString,
    /// The version of the interface to be constructed.
    version: u32,
    /// The id of the new object.
    id: NewId,
}

impl const MessageSize for ComptimeGenericNewId {
    fn size(&self) -> usize {
        self.interface.size() + self.version.size() + self.id.size()
    }
}

/// A string to be serialized.
// A wayland string is a null-terminated UTF-8 string prefixed by a 32-bit length, which includes the null terminator.
pub struct String<'a>(Cow<'a, [u8]>);

impl<'a> MessageSize for String<'_> {
    fn size(&self) -> usize {
        pad_to_32_bits(self.0.len() + 1) + 4
    }
}

/// A wayland string that is known at compile time, faster to serialize.
pub struct ComptimeString(&'static str);

impl const MessageSize for ComptimeString {
    fn size(&self) -> usize {
        pad_to_32_bits(self.0.len() + 1) + 4
    }
}

/// A wayland array, which is a sequence of bytes prefixed by a 32-bit length on the wire.
pub struct Array<'a>(Cow<'a, [u8]>);

impl<'a> MessageSize for Array<'_> {
    fn size(&self) -> usize {
        pad_to_32_bits(self.0.len()) + 4
    }
}

/// The various errors that can occur while serializing or deserializing.
#[derive(Debug, Error)]
pub enum SerdeError {
    /// An IO error occurred while encoding/decoding.
    #[error("IO error occurred while decoding")]
    IoError(#[from] std::io::Error),
}
