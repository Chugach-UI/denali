//! Connection types and traits.

use crate::{
    Interface,
    handler::Handler,
    id::ObjectId,
    message::{Event, MessageTypeMarker, OutgoingMessage, Request},
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
pub trait Connection<'a> {
    type Error;

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

    /// Add a handler to an object.
    ///
    /// On a client connection, the handler will be called when the object receives an event.
    /// On a server connection, the handler will be called when the object receives a request.
    fn add_handler<I: Interface, H: Handler + 'a>(&mut self, object: &ObjectId<I>, handler: H);

    async fn send_message<M: MessageTypeMarker, O: OutgoingMessage<M>>(
        &mut self,
        message: O,
    ) -> Result<O::Response, Self::Error>;

    async fn send_event<O: OutgoingMessage<Event>>(
        &mut self,
        message: O,
    ) -> Result<O::Response, Self::Error> {
        self.send_message(message).await
    }
    async fn send_request<O: OutgoingMessage<Request>>(
        &mut self,
        message: O,
    ) -> Result<O::Response, Self::Error> {
        self.send_message(message).await
    }
}
