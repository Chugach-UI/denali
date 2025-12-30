//! Traits and utilities for handling requests and events.

use crate::{
    Interface,
    id::AnyObjectId,
    message::{IncomingMessage, MessageCoprod, MessageType},
};

/// A trait for handling Wayland messages.
pub trait Handler: crate::sealed::Sealed {
    fn handle_message(
        &mut self,
        interface: &str,
        opcode: u16,
        message_type: MessageType,
        data: &[u8],
        id: AnyObjectId,
    );
}

pub struct EventHandler<'a, I: Interface, F: FnMut(I::Event<'a>)> {
    handler: F,
    _marker: std::marker::PhantomData<(I, fn(&'a ()))>,
}

impl<'a, I: Interface, F: FnMut(I::Event<'a>)> EventHandler<'a, I, F> {
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            _marker: std::marker::PhantomData,
        }
    }
}
impl<'a, I: Interface, F: FnMut(I::Event<'a>)> crate::sealed::Sealed for EventHandler<'a, I, F> {}

impl<'a, I: Interface, F: FnMut(I::Event<'a>)> Handler for EventHandler<'a, I, F> {
    fn handle_message(
        &mut self,
        interface: &str,
        opcode: u16,
        message_type: MessageType,
        data: &[u8],
        id: AnyObjectId,
    ) {
        let Ok(message) = MessageCoprod::<I::Event<'a>, I::Request<'a>>::try_decode(
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

pub struct RequestHandler<'a, I: Interface, F: FnMut(I::Request<'a>)> {
    handler: F,
    _marker: std::marker::PhantomData<(I, fn(&'a ()))>,
}
impl<'a, I: Interface, F: FnMut(I::Request<'a>)> crate::sealed::Sealed
    for RequestHandler<'a, I, F>
{
}

impl<'a, I: Interface, F: FnMut(I::Request<'a>)> RequestHandler<'a, I, F> {
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<'a, I: Interface, F: FnMut(I::Request<'a>)> Handler for RequestHandler<'a, I, F> {
    fn handle_message(
        &mut self,
        interface: &str,
        opcode: u16,
        message_type: MessageType,
        data: &[u8],
        id: AnyObjectId,
    ) {
        let Ok(message) = MessageCoprod::<I::Event<'a>, I::Request<'a>>::try_decode(
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
