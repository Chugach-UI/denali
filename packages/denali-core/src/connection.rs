//! Connection types and traits.

use crate::{
    message::{
        Event, IncomingMessage, MessageTypeMarker, OutgoingMessage, Request,
    },
    wire::serde::MessageHeader,
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
pub trait ClientConnection: Connection {
    /// Send a request to the remote endpoint.
    async fn send_request<O: OutgoingMessage<Request>>(
        &mut self,
        message: O,
    ) -> Result<O::Response, Self::Error>;
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
pub trait ServerConnection: Connection {
    /// Send an event to the remote endpoint.
    async fn send_event<O: OutgoingMessage<Event>>(
        &mut self,
        message: O,
    ) -> Result<O::Response, Self::Error>;
}
impl<T: Connection<IncomingMessageType = Request>> ServerConnection for T {
    async fn send_event<O: OutgoingMessage<Event>>(
        &mut self,
        message: O,
    ) -> Result<O::Response, Self::Error> {
        self.send_message(message).await
    }
}
