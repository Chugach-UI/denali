//! Traits and utilities for handling requests and events.

use frunk::{Coproduct, coproduct::CNil};
use thiserror::Error;

use crate::wire::serde::ObjectId;

/// Represents a message (either request or event) sent over the wire that can be decoded and handled.
///
/// The `try_decode` method provides the targeted object's interface name, the opcode, and the body of the message
pub trait Message {
    /// Attempt to decode a message from the given interface name, opcode, and data.
    ///
    /// # Errors
    ///
    /// This method can return the following errors:
    /// - [`DecodeMessageError::UnknownInterface`]: The provided interface name is not recognized.
    /// - [`DecodeMessageError::UnknownOpcode`]: The provided opcode is not recognized for the given interface.
    /// - [`DecodeMessageError::DecodeError`]: The message could not be decoded due to malformed data.
    fn try_decode(interface: &str, opcode: u16, data: &[u8]) -> Result<Self, DecodeMessageError>
    where
        Self: Sized;
}

/// A handler for messages of type `M`.
///
/// The `handle` method is called when a message of type `M` is received, along with the ID of the object the message is associated with.
pub trait Handler<M: Message> {
    /// Handle a message of type `M` associated with the given object ID.
    fn handle(&mut self, message: M, object_id: ObjectId);
}

impl<A: Message, B: Message> Message for Coproduct<A, B> {
    fn try_decode(interface: &str, opcode: u16, data: &[u8]) -> Result<Self, DecodeMessageError> {
        match A::try_decode(interface, opcode, data) {
            Ok(msg) => return Ok(Self::Inl(msg)),
            Err(DecodeMessageError::UnknownInterface(_)) => {}
            Err(e) => return Err(e),
        }
        B::try_decode(interface, opcode, data).map(Self::Inr)
    }
}
impl Message for CNil {
    fn try_decode(interface: &str, _opcode: u16, _data: &[u8]) -> Result<Self, DecodeMessageError> {
        Err(DecodeMessageError::UnknownInterface(interface.to_string()))
    }
}
impl<T> Handler<CNil> for T {
    fn handle(&mut self, _message: CNil, _object_id: ObjectId) {}
}

impl<L: Message, R: Message, H: Handler<L> + Handler<R>> Handler<Coproduct<L, R>> for H {
    fn handle(&mut self, message: Coproduct<L, R>, object_id: ObjectId) {
        match message {
            Coproduct::Inl(l) => self.handle(l, object_id),
            Coproduct::Inr(r) => self.handle(r, object_id),
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
