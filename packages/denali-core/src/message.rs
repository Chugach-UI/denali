use thiserror::Error;

use crate::{
    Interface,
    id::{IdFactory, ObjectId},
    sealed,
    wire::serde::{CompileTimeMessageSize, Encode, MessageHeader, MessageSize, SerdeError},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageType {
    Event,
    Request,
}

pub trait MessageTypeMarker: sealed::Sealed {
    const EVENT: bool;
    const REQUEST: bool;
    type Complement: MessageTypeMarker;
}
pub struct Event(());
pub struct Request(());
pub(crate) struct EventOrRequest(());
pub(crate) struct NeitherEventNorRequest(());
impl sealed::Sealed for Event {}
impl sealed::Sealed for Request {}
impl sealed::Sealed for EventOrRequest {}
impl sealed::Sealed for NeitherEventNorRequest {}
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
impl MessageTypeMarker for EventOrRequest {
    const EVENT: bool = true;
    const REQUEST: bool = true;
    type Complement = NeitherEventNorRequest;
}
impl MessageTypeMarker for NeitherEventNorRequest {
    const EVENT: bool = false;
    const REQUEST: bool = false;
    type Complement = EventOrRequest;
}

/// Represents a message (either request or event) incoming over the wayland wire.
pub trait IncomingMessage<T: MessageTypeMarker> {
    type Interface: Interface;

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
        data: &[u8],
    ) -> Result<Self, DecodeMessageError>
    where
        Self: Sized;
}

/// Represents a message (either request or event) outgoing over the wayland wire.
pub trait OutgoingMessage<T: MessageTypeMarker>: EncodeWithNewId {
    type Interface: Interface;
    const OPCODE: u16;

    /// The type of response expected from this event/request.
    type Response: MessageResponse;

    fn sender(&self) -> &ObjectId<Self::Interface>;
}

pub trait MessageResponse {
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

pub trait EncodeWithNewId: MessageSize {
    /// Encodes this instance into the provided byte slice.
    ///
    /// # Errors
    ///
    /// This function returns errors if:
    /// - The provided data slice is not large enough to contain the encoded type.
    /// - An IO error occurs while writing to the data slice.
    /// - An invalid enum value is encountered while encoding an enum type.
    fn encode(&self, data: &mut [u8], id_factory: IdFactory<'_>) -> Result<usize, SerdeError>;
}

pub struct NewIdHint<I: Interface>(std::marker::PhantomData<I>);
impl<I: Interface> NewIdHint<I> {
    #[must_use]
    pub const fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

pub(crate) enum MessageCoprod<E, R> {
    Event(E),
    Request(R),
}
impl<E, R> MessageCoprod<E, R> {
    pub const fn new_event(event: E) -> Self {
        Self::Event(event)
    }

    pub const fn new_request(request: R) -> Self {
        Self::Request(request)
    }

    pub fn into_event(self) -> Option<E> {
        match self {
            Self::Event(event) => Some(event),
            Self::Request(_) => None,
        }
    }

    pub fn into_request(self) -> Option<R> {
        match self {
            Self::Event(_) => None,
            Self::Request(request) => Some(request),
        }
    }
}

impl<T, U> IncomingMessage<EventOrRequest> for MessageCoprod<T, U>
where
    T: IncomingMessage<Event>,
    U: IncomingMessage<Request, Interface = T::Interface>,
{
    type Interface = T::Interface;

    fn try_decode(
        interface: &str,
        opcode: u16,
        message_type: MessageType,
        data: &[u8],
    ) -> Result<Self, DecodeMessageError>
    where
        Self: Sized,
    {
        let target_interface: &str = Self::Interface::INTERFACE;

        if interface != target_interface {
            return Err(DecodeMessageError::UnknownInterface(interface.to_string()));
        }

        let is_event = matches!(message_type, MessageType::Event);

        if is_event {
            T::try_decode(interface, opcode, message_type, data).map(Self::new_event)
        } else {
            U::try_decode(interface, opcode, message_type, data).map(Self::new_request)
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
) -> Result<usize, SerdeError> {
    let header = MessageHeader {
        object_id,
        size: (MessageHeader::SIZE + message.size()) as u16,
        opcode,
    };
    header.encode(&mut data[..])?;
    let encoded_size = message.encode(&mut data[MessageHeader::SIZE..], id_factory)?;

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
