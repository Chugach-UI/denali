//! Traits and utilities for handling requests and events.

use std::pin::Pin;

use async_trait::async_trait;

use crate::{
    Interface,
    id::{AnyObjectId, BorrowedObjectId, ObjectId},
    message::{Event, IncomingMessage, MessageCoprod, MessageType, MessageTypeMarker, Request},
    prelude::Connection,
};

/// A trait for handling Wayland messages.
#[async_trait(?Send)]
pub trait Handler<M: MessageTypeMarker>: crate::sealed::Sealed {
    async fn handle_message(&mut self, opcode: u16, data: &[u8], id: AnyObjectId);
}

pub struct EventHandler<I: Interface, F: AsyncFnMut(I::Event<'static>, BorrowedObjectId<I>) -> ()> {
    handler: F,
    _marker: std::marker::PhantomData<I>,
}

impl<I: Interface, F: AsyncFnMut(I::Event<'static>, BorrowedObjectId<I>) -> ()> EventHandler<I, F> {
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            _marker: std::marker::PhantomData,
        }
    }
}
impl<I: Interface, F: AsyncFnMut(I::Event<'static>, BorrowedObjectId<I>) -> ()>
    crate::sealed::Sealed for EventHandler<I, F>
{
}

#[async_trait(?Send)]
impl<I: Interface, F: AsyncFnMut(I::Event<'static>, BorrowedObjectId<I>) -> ()> Handler<Event>
    for EventHandler<I, F>
{
    async fn handle_message(&mut self, opcode: u16, data: &[u8], id: AnyObjectId) {
        let Ok(message) = MessageCoprod::<I::Event<'_>, I::Request<'_>>::try_decode(
            I::INTERFACE,
            opcode,
            MessageType::Event,
            data,
        ) else {
            return;
        };
        let Some(event) = message.into_event() else {
            return;
        };

        let id = unsafe { BorrowedObjectId::new(ObjectId::new(id)) };

        (self.handler)(event, id).await;
    }
}

pub fn event_handler<I: Interface, F: AsyncFnMut(I::Event<'static>, BorrowedObjectId<I>) -> ()>(
    handler: F,
) -> EventHandler<I, F> {
    EventHandler {
        handler,
        _marker: std::marker::PhantomData,
    }
}

pub struct RequestHandler<
    I: Interface,
    F: AsyncFnMut(I::Request<'static>, BorrowedObjectId<I>) -> (),
> {
    handler: F,
    _marker: std::marker::PhantomData<I>,
}

impl<I: Interface, F: AsyncFnMut(I::Request<'static>, BorrowedObjectId<I>) -> ()>
    RequestHandler<I, F>
{
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            _marker: std::marker::PhantomData,
        }
    }
}
impl<I: Interface, F: AsyncFnMut(I::Request<'static>, BorrowedObjectId<I>) -> ()>
    crate::sealed::Sealed for RequestHandler<I, F>
{
}

#[async_trait(?Send)]
impl<I: Interface, F: AsyncFnMut(I::Request<'static>, BorrowedObjectId<I>) -> ()> Handler<Request>
    for RequestHandler<I, F>
{
    async fn handle_message(&mut self, opcode: u16, data: &[u8], id: AnyObjectId) {
        let Ok(message) = MessageCoprod::<I::Event<'_>, I::Request<'_>>::try_decode(
            I::INTERFACE,
            opcode,
            MessageType::Request,
            data,
        ) else {
            return;
        };
        let Some(request) = message.into_request() else {
            return;
        };

        let id = unsafe { BorrowedObjectId::new(ObjectId::new(id)) };

        (self.handler)(request, id).await;
    }
}
