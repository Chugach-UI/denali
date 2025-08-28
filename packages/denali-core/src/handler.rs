//! Traits and utilities for handling requests and events.

use thiserror::Error;

use crate::wire::serde::ObjectId;

pub trait Message {
    fn try_decode(interface: &str, opcode: u16, data: &[u8]) -> Result<Self, DecodeMessageError>
    where
        Self: Sized;
}

pub trait Handler<M: Message> {
    fn handle(&mut self, message: M, object_id: ObjectId);
}

/// A union of two message types, used for proving that one type can handle multiple types.
pub enum MessageUnion<A: Message, B: Message> {
    A(A),
    B(B),
}
impl<A: Message, B: Message> Message for MessageUnion<A, B> {
    fn try_decode(interface: &str, opcode: u16, data: &[u8]) -> Result<Self, DecodeMessageError> {
        A::try_decode(interface, opcode, data)
            .map(Self::A)
            .or_else(|_| B::try_decode(interface, opcode, data).map(Self::B))
    }
}

impl<A: Message, B: Message, H: Handler<A> + Handler<B>> Handler<MessageUnion<A, B>> for H {
    fn handle(&mut self, message: MessageUnion<A, B>, object_id: ObjectId) {
        match message {
            MessageUnion::A(a) => self.handle(a, object_id),
            MessageUnion::B(b) => self.handle(b, object_id),
        }
    }
}

#[derive(Debug, Error)]
pub enum DecodeMessageError {
    #[error("unknown interface: {0}")]
    UnknownInterface(String),
    #[error("unknown opcode: {0}")]
    UnknownOpcode(u16),
    #[error("failed to decode message: {0}")]
    DecodeError(#[from] crate::wire::serde::SerdeError),
}
