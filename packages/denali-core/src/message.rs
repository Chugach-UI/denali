//! Wayland message-related types and traits.

use std::os::fd::RawFd;

use thiserror::Error;

use crate::{
    Interface,
    id::{IdFactory, ObjectId},
    sealed,
    wire::serde::{
        CompileTimeMessageSize, Encode, MessageHeader, MessageSize, RawObjectId, SerdeError,
    },
};

/// Represents the direction of a message over the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageType {
    /// An event message.
    Event,
    /// A request message.
    Request,
}

/// Trait representing the direction of a message over the wire.
pub trait MessageTypeMarker: sealed::Sealed {
    /// Whether this message type is an event.
    const EVENT: bool;
    /// Whether this message type is a request.
    const REQUEST: bool;
    /// The opposite direction of this message type.
    type Complement: MessageTypeMarker;
}
/// Marker type for event messages.
pub struct Event(());
/// Marker type for request messages.
pub struct Request(());
impl sealed::Sealed for Event {}
impl sealed::Sealed for Request {}
impl MessageTypeMarker for Event {
    const EVENT: bool = true;
    const REQUEST: bool = false;
    type Complement = Request;
}
impl MessageTypeMarker for Request {
    const EVENT: bool = false;
    const REQUEST: bool = true;
    type Complement = Event;
}

/// A raw (undecoded) message received over the wayland wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawWaylandMessage {
    /// The object to receive the message.
    pub object_id: RawObjectId,
    /// The opcode of the message.
    pub opcode: u16,
    /// The data of the message.
    pub body: Vec<u8>,
}

/// Represents a message (either request or event) incoming over the wayland wire.
pub trait IncomingMessage<'de, T: MessageTypeMarker>: Sized {
    /// The type of interface associated with this message.
    type Interface: Interface;

    /// The number of file descriptors associated with this message.
    fn fd_count(_opcode: u16) -> usize {
        0
    }

    /// Attempt to decode a message from the given interface name, opcode, and data.
    ///
    /// # Errors
    ///
    /// This method can return the following errors:
    /// - [`DecodeMessageError::UnknownInterface`]: The provided interface name is not recognized.
    /// - [`DecodeMessageError::UnknownOpcode`]: The provided opcode is not recognized for the given interface.
    /// - [`DecodeMessageError::DecodeError`]: The message could not be decoded due to malformed data.
    fn try_decode(
        interface: &str,
        opcode: u16,
        message_type: MessageType,
        data: &'de [u8],
        fds: &[RawFd],
    ) -> Result<Self, DecodeMessageError>;
}

/// Represents a message (either request or event) outgoing over the wayland wire.
pub trait OutgoingMessage<T: MessageTypeMarker>: EncodeWithNewId {
    /// The interface associated with this message.
    type Interface: Interface;
    /// The opcode of the message.
    const OPCODE: u16;
    /// The number of file descriptors associated with this message.
    const FD_COUNT: usize = 0;

    /// The type of response expected from this event/request.
    type Response: MessageResponse;

    /// Returns the id of the object sending this message.
    fn sender(&self) -> &ObjectId<Self::Interface>;
}

/// A trait implemented by possible responses to a message.
pub trait MessageResponse {
    /// Create a new instance of this response with a provided [`IdFactory`].
    fn with_id_factory(id_factory: IdFactory<'_>) -> Self;
}
impl MessageResponse for () {
    fn with_id_factory(_id_factory: IdFactory<'_>) -> Self {
        ();
    }
}
impl<I: Interface> MessageResponse for ObjectId<I> {
    fn with_id_factory(mut id_factory: IdFactory<'_>) -> Self {
        unsafe { id_factory.alloc_typed_id().unwrap() }
    }
}

/// A trait for message types that need to create/peek new IDs while encoding.
pub trait EncodeWithNewId: MessageSize {
    /// Encodes this instance into the provided byte slice.
    ///
    /// # Errors
    ///
    /// This function returns errors if:
    /// - The provided data slice is not large enough to contain the encoded type.
    /// - An IO error occurs while writing to the data slice.
    /// - An invalid enum value is encountered while encoding an enum type.
    fn encode(
        &self,
        data: &mut [u8],
        id_factory: IdFactory<'_>,
        fds: &mut [RawFd],
    ) -> Result<usize, SerdeError>;
}

/// A hint used to allocate new IDs for a specific interface.
///
/// # Example
///
/// This hint is used in a `WlRegistryBindRequest` to type the returned object ID.
pub struct NewIdHint<I: Interface> {
    /// Version of the interface to be created
    pub version: u32,
    _marker: std::marker::PhantomData<I>,
}

impl<I: Interface> NewIdHint<I> {
    /// Creates a new `NewIdHint` instance.
    #[must_use]
    pub const fn new(version: u32) -> Self {
        Self {
            version,
            _marker: std::marker::PhantomData,
        }
    }
}

/// Encodes a message with the given object ID and opcode into the provided byte buffer.
///
/// # Errors
///
/// Returns an error if encoding fails. See [`Encode::encode`](serde::Encode::encode) for more details.
pub fn encode_message<T: EncodeWithNewId>(
    message: &T,
    object_id: u32,
    opcode: u16,
    data: &mut [u8],
    id_factory: IdFactory<'_>,
    fds: &mut [RawFd],
) -> Result<usize, SerdeError> {
    let header = MessageHeader {
        object_id,
        size: (MessageHeader::SIZE + message.size()) as u16,
        opcode,
    };
    header.encode(&mut data[..])?;
    let encoded_size = message.encode(&mut data[MessageHeader::SIZE..], id_factory, fds)?;

    let final_size = MessageHeader::SIZE + encoded_size;

    Ok(final_size)
}

/// Errors that can occur while decoding a message.
#[derive(Debug, Error)]
pub enum DecodeMessageError {
    /// The provided interface name is not recognized.
    #[error("unknown interface: {0}")]
    UnknownInterface(String),
    /// The provided opcode is not recognized for the given interface.
    #[error("unknown opcode: {0}")]
    UnknownOpcode(u16),
    /// The message could not be decoded due to malformed data.
    #[error("failed to decode message: {0}")]
    DecodeError(#[from] crate::wire::serde::SerdeError),
}

/// Errors that can occur while encoding a message.
#[derive(Debug, Error)]
pub enum EncodeMessageError {
    /// The message could not be encoded due to malformed data.
    #[error("failed to encode message: {0}")]
    EncodeError(#[from] crate::wire::serde::SerdeError),
}
