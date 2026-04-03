//! Connection types and traits.

use crate::{
    Interface, id::ObjectId, message::{
        Event, IncomingMessage, MessageTypeMarker, OutgoingMessage, Request,
    }, wire::serde::MessageHeader
};

/// Connection types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    /// Client end of a connection.
    Client,
    /// Server end of a connection.
    Server,
}
impl ConnectionType {
    /// Returns true if the connection type is client.
    #[must_use]
    pub fn is_client(&self) -> bool {
        *self == ConnectionType::Client
    }
    /// Returns true if the connection type is server.
    #[must_use]
    pub fn is_server(&self) -> bool {
        *self == ConnectionType::Server
    }
}

/// A trait implemented by both client and server connections.
///
/// This trait provides methods for managing handlers, sending requests, and sending events.
#[allow(async_fn_in_trait)]
pub trait Connection {
    /// The type of error that can occur when sending or receiving messages.
    type Error;
    /// The type of incoming message.
    /// For client connections, this will be [`Event`](crate::message::Event).
    /// For server connections, this will be [`Request`](crate::message::Request).
    type IncomingMessageType: MessageTypeMarker;

    /// Returns the type of the connection.
    fn connection_type(&self) -> ConnectionType;

    /// Returns true if the connection is a client.
    fn is_client(&self) -> bool {
        self.connection_type().is_client()
    }
    /// Returns true if the connection is a server.
    fn is_server(&self) -> bool {
        self.connection_type().is_server()
    }

    /// Send a message to the remote endpoint.
    async fn send_message<
        O: OutgoingMessage<<Self::IncomingMessageType as MessageTypeMarker>::Complement>,
    >(
        &mut self,
        message: O,
    ) -> Result<O::Response, Self::Error>;

    /// Receive the next message from the remote endpoint.
    async fn next_header(&mut self) -> Result<MessageHeader, Self::Error>;
    /// Decode the next message from the remote endpoint.
    ///
    /// The returned message may borrow from the connection's internal buffer.
    /// The buffer is reclaimed on the next call to this method or [`next_header`](Self::next_header).
    async fn decode_message<'a, M: IncomingMessage<'a, Self::IncomingMessageType>>(
        &'a mut self,
    ) -> Result<M, Self::Error>;
}

/// Extension trait for client-sided connections.
#[allow(async_fn_in_trait)]
pub trait ClientConnection: Connection<IncomingMessageType = Event> {
    /// Send a request to the remote endpoint.
    async fn send_request<O: OutgoingMessage<Request>>(
        &mut self,
        message: O,
    ) -> Result<O::Response, Self::Error>;

    /// Receive the next event for a specific object, skipping events for other objects.
    ///
    /// The event type is inferred from the object's interface. Loops internally
    /// until a message arrives for `object`, then decodes and returns it.
    /// Messages for other objects are silently discarded.
    async fn recv_event<'a, I: Interface>(
        &'a mut self,
        object: &ObjectId<I>,
    ) -> Result<I::Event<'a>, Self::Error> {
        let id = object.get();
        loop {
            let head = self.next_header().await?;
            if head.object_id == id {
                return self.decode_message().await;
            }
        }
    }

    /// Try to decode the current message as an event for `object`.
    ///
    /// Call this after [`next_header`](denali_core::connection::Connection::next_header)
    /// to conditionally decode the peeked message. Returns `Some(event)` if the
    /// header's object ID matches, or `None` without consuming the message.
    ///
    /// Chain multiple calls to dispatch events from different objects:
    /// ```ignore
    /// loop {
    ///     let head = conn.next_header().await?;
    ///     if let Some(event) = conn.try_recv_event(&head, &xdg_surface).await? {
    ///         // handle xdg_surface events
    ///     } else if let Some(event) = conn.try_recv_event(&head, &output).await? {
    ///         // handle wl_output events
    ///     }
    /// }
    /// ```
    async fn try_recv_event<'a, I: Interface>(
        &'a mut self,
        header: &MessageHeader,
        object: &ObjectId<I>,
    ) -> Result<Option<I::Event<'a>>, Self::Error> {
        if header.object_id == object.get() {
            let event = self.decode_message().await?;
            Ok(Some(event))
        } else {
            Ok(None)
        }
    }
}
impl<T: Connection<IncomingMessageType = Event>> ClientConnection for T {
    async fn send_request<O: OutgoingMessage<Request>>(
        &mut self,
        message: O,
    ) -> Result<O::Response, Self::Error> {
        self.send_message(message).await
    }
}

/// Extension trait for server-sided connections.
#[allow(async_fn_in_trait)]
pub trait ServerConnection: Connection<IncomingMessageType = Request> {
    /// Send an event to the remote endpoint.
    async fn send_event<O: OutgoingMessage<Event>>(
        &mut self,
        message: O,
    ) -> Result<O::Response, Self::Error>;

    /// Receive the next request for a specific object, skipping requests for other objects.
    ///
    /// The request type is inferred from the object's interface. Loops internally
    /// until a message arrives for `object`, then decodes and returns it.
    /// Messages for other objects are silently discarded.
    async fn recv_request<'a, I: Interface>(
        &'a mut self,
        object: &ObjectId<I>,
    ) -> Result<I::Request<'a>, Self::Error> {
        let id = object.get();
        loop {
            let head = self.next_header().await?;
            if head.object_id == id {
                return self.decode_message().await;
            }
        }
    }

    /// Try to decode the current message as a request for `object`.
    ///
    /// Call this after [`next_header`](Connection::next_header)
    /// to conditionally decode the peeked message. Returns `Some(request)` if the
    /// header's object ID matches, or `None` without consuming the message.
    ///
    /// Chain multiple calls to dispatch requests from different objects:
    /// ```ignore
    /// loop {
    ///     let head = conn.next_header().await?;
    ///     if let Some(req) = conn.try_recv_request(&head, &surface).await? {
    ///         // handle wl_surface requests
    ///     } else if let Some(req) = conn.try_recv_request(&head, &seat).await? {
    ///         // handle wl_seat requests
    ///     }
    /// }
    /// ```
    async fn try_recv_request<'a, I: Interface>(
        &'a mut self,
        header: &MessageHeader,
        object: &ObjectId<I>,
    ) -> Result<Option<I::Request<'a>>, Self::Error> {
        if header.object_id == object.get() {
            let request = self.decode_message().await?;
            Ok(Some(request))
        } else {
            Ok(None)
        }
    }
}
impl<T: Connection<IncomingMessageType = Request>> ServerConnection for T {
    async fn send_event<O: OutgoingMessage<Event>>(
        &mut self,
        message: O,
    ) -> Result<O::Response, Self::Error> {
        self.send_message(message).await
    }
}
