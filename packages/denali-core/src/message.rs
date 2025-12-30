use thiserror::Error;

use crate::{
    Interface,
    id::{IdFactory, ObjectId},
    sealed,
    wire::serde::{Encode, MessageSize, SerdeError},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageType {
    Event,
    Request,
}

pub trait MessageTypeMarker: sealed::Sealed {
    const EVENT: bool;
    const REQUEST: bool;
}
pub struct Event(());
pub struct Request(());
pub(crate) struct EventOrRequest(());
impl sealed::Sealed for Event {}
impl sealed::Sealed for Request {}
impl sealed::Sealed for EventOrRequest {}
impl MessageTypeMarker for Event {
    const EVENT: bool = true;
    const REQUEST: bool = false;
}
impl MessageTypeMarker for Request {
    const EVENT: bool = false;
    const REQUEST: bool = true;
}
impl MessageTypeMarker for EventOrRequest {
    const EVENT: bool = true;
    const REQUEST: bool = true;
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
    type Response;

    fn sender(&self) -> &ObjectId<Self::Interface>;
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

pub(crate) enum MessageCoprod<E, R> {
    Event(E),
    Request(R),
}
impl<E, R> MessageCoprod<E, R> {
    pub fn new_event(event: E) -> Self {
        Self::Event(event)
    }

    pub fn new_request(request: R) -> Self {
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
