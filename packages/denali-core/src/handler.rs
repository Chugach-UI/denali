//! Traits and utilities for handling requests and events.

use crate::{
    Interface,
    id::{BorrowedObjectId, DynamicObjectId},
    message::{IncomingMessage, MessageCoprod, MessageType},
};

/// A trait for handling Wayland messages.
pub trait Handler: crate::sealed::Sealed {
    fn handle_message<'a>(
        &mut self,
        interface: &str,
        opcode: u16,
        message_type: MessageType,
        data: &[u8],
        id: DynamicObjectId,
    );
}

pub struct EventHandler<I: Interface, F: FnMut(I::Event)> {
    handler: F,
    _marker: std::marker::PhantomData<I>,
}

impl<I: Interface, F: FnMut(I::Event)> EventHandler<I, F> {
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            _marker: std::marker::PhantomData,
        }
    }
}
impl<I: Interface, F: FnMut(I::Event)> crate::sealed::Sealed for EventHandler<I, F> {}

impl<I: Interface, F: FnMut(I::Event)> Handler for EventHandler<I, F> {
    fn handle_message<'a>(
        &mut self,
        interface: &str,
        opcode: u16,
        message_type: MessageType,
        data: &[u8],
        id: DynamicObjectId,
    ) {
        let Ok(message) = MessageCoprod::<I::Event, I::Request>::try_decode(
            interface,
            opcode,
            message_type,
            data,
        ) else {
            return;
        };
        let Some(event) = message.into_event() else {
            return;
        };
        (self.handler)(event);
    }
}

pub struct RequestHandler<I: Interface, F: FnMut(I::Request)> {
    handler: F,
    _marker: std::marker::PhantomData<I>,
}
impl<I: Interface, F: FnMut(I::Request)> crate::sealed::Sealed for RequestHandler<I, F> {}

impl<I: Interface, F: FnMut(I::Request)> RequestHandler<I, F> {
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<I: Interface, F: FnMut(I::Request)> Handler for RequestHandler<I, F> {
    fn handle_message<'a>(
        &mut self,
        interface: &str,
        opcode: u16,
        message_type: MessageType,
        data: &[u8],
        id: DynamicObjectId,
    ) {
        let Ok(message) = MessageCoprod::<I::Event, I::Request>::try_decode(
            interface,
            opcode,
            message_type,
            data,
        ) else {
            return;
        };
        let Some(event) = message.into_request() else {
            return;
        };
        (self.handler)(event);
    }
}
