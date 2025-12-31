//! Traits and utilities for handling requests and events.

use std::pin::Pin;

use async_trait::async_trait;

use crate::{
    Interface,
    id::{AnyObjectId, ObjectId},
    message::{IncomingMessage, MessageCoprod, MessageType},
    prelude::Connection,
};

/// A trait for handling Wayland messages.
#[async_trait(?Send)]
pub trait Handler<'c>: crate::sealed::Sealed {
    type Connection: Connection<'c>;
    async fn handle_message(
        &mut self,
        opcode: u16,
        message_type: MessageType,
        data: &[u8],
        id: AnyObjectId,
        connection: &mut Self::Connection,
    );
}

pub struct EventHandler<'c, I: Interface, C: Connection<'c>, F>
where
    for<'r> F:
        Fn(I::Event<'static>, &'r mut C, &'r ObjectId<I>) -> Pin<Box<dyn Future<Output = ()> + 'r>>,
{
    handler: F,
    _marker: std::marker::PhantomData<(I, &'c (), fn(C))>,
}

impl<'c, I: Interface, C: Connection<'c>, F> EventHandler<'c, I, C, F>
where
    for<'r> F:
        Fn(I::Event<'static>, &'r mut C, &'r ObjectId<I>) -> Pin<Box<dyn Future<Output = ()> + 'r>>,
{
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            _marker: std::marker::PhantomData,
        }
    }
}
impl<'c, I: Interface, C: Connection<'c>, F> crate::sealed::Sealed for EventHandler<'c, I, C, F> where
    for<'r> F:
        Fn(I::Event<'static>, &'r mut C, &'r ObjectId<I>) -> Pin<Box<dyn Future<Output = ()> + 'r>>
{
}

#[async_trait(?Send)]
impl<'c, I: Interface, C: Connection<'c>, F> Handler<'c> for EventHandler<'c, I, C, F>
where
    for<'r> F:
        Fn(I::Event<'static>, &'r mut C, &'r ObjectId<I>) -> Pin<Box<dyn Future<Output = ()> + 'r>>,
{
    type Connection = C;

    async fn handle_message(
        &mut self,
        opcode: u16,
        message_type: MessageType,
        data: &[u8],
        id: AnyObjectId,
        connection: &mut C,
    ) {
        let Ok(message) = MessageCoprod::<I::Event<'_>, I::Request<'_>>::try_decode(
            I::INTERFACE,
            opcode,
            message_type,
            data,
        ) else {
            return;
        };
        let Some(event) = message.into_event() else {
            return;
        };

        let id = unsafe { ObjectId::new(id) };

        (self.handler)(event, connection, &id).await;
    }
}

pub struct RequestHandler<
    'a,
    I: Interface,
    C: Connection<'a>,
    F: AsyncFnMut(I::Request<'_>, &mut C, &ObjectId<I>),
> {
    handler: F,
    _marker: std::marker::PhantomData<(I, fn(&'a C))>,
}

impl<'a, I: Interface, C: Connection<'a>, F: AsyncFnMut(I::Request<'_>, &mut C, &ObjectId<I>)>
    RequestHandler<'a, I, C, F>
{
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            _marker: std::marker::PhantomData,
        }
    }
}
impl<'a, I: Interface, C: Connection<'a>, F: AsyncFnMut(I::Request<'_>, &mut C, &ObjectId<I>)>
    crate::sealed::Sealed for RequestHandler<'a, I, C, F>
{
}

#[async_trait(?Send)]
impl<'a, I: Interface, C: Connection<'a>, F: AsyncFnMut(I::Request<'_>, &mut C, &ObjectId<I>)>
    Handler<'a> for RequestHandler<'a, I, C, F>
{
    type Connection = C;

    async fn handle_message(
        &mut self,
        opcode: u16,
        message_type: MessageType,
        data: &[u8],
        id: AnyObjectId,
        connection: &mut Self::Connection,
    ) {
        let Ok(message) = MessageCoprod::<I::Event<'_>, I::Request<'_>>::try_decode(
            I::INTERFACE,
            opcode,
            message_type,
            data,
        ) else {
            return;
        };
        let Some(request) = message.into_request() else {
            return;
        };

        let id = unsafe { ObjectId::new(id) };

        (self.handler)(request, connection, &id).await;
    }
}
