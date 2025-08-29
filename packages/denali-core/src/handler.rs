//! Traits and utilities for handling requests and events.

use std::cell::{Cell, RefCell};

use frunk::{coproduct::CNil, hlist, Coprod, Coproduct};
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

impl<A: Message, B: Message> Message for Coproduct<A, B> {
    fn try_decode(interface: &str, opcode: u16, data: &[u8]) -> Result<Self, DecodeMessageError> {
        A::try_decode(interface, opcode, data)
            .map(Self::Inl)
            .or_else(|_| B::try_decode(interface, opcode, data).map(Self::Inr))
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

#[derive(Debug, Error)]
pub enum DecodeMessageError {
    #[error("unknown interface: {0}")]
    UnknownInterface(String),
    #[error("unknown opcode: {0}")]
    UnknownOpcode(u16),
    #[error("failed to decode message: {0}")]
    DecodeError(#[from] crate::wire::serde::SerdeError),
}
