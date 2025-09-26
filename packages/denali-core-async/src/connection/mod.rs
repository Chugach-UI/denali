//! An async connection to a wayland server, powered by tokio io.

use std::os::fd::OwnedFd;

use thiserror::Error;
use tokio::io::unix::AsyncFd;
mod stream;

/// A thread-safe wrapper around an async wayland connection.
pub struct AsyncConnection(AsyncConnectionInner);

impl AsyncConnection {
    /// Creates a new async connection to a wayland server.
    pub fn new(socket: OwnedFd) -> Result<Self, AsyncConnectionError> {
        let socket = AsyncFd::new(socket);
        todo!()
    }

    /// Sends a message to the wayland server without ancillary data.
    pub async fn send_message(&self, message: &[u8]) -> Result<(), AsyncConnectionError> {
        self.0.send_message(message).await
    }
}

/// An async wayland connection.
pub struct AsyncConnectionInner {
    /// The write stream for the connection.
    write_stream: stream::AsyncWriteStream,
}

impl AsyncConnectionInner {
    /// Sends a message to the wayland server without ancillary data.
    pub async fn send_message(&self, message: &[u8]) -> Result<(), AsyncConnectionError> {
        self.write_stream
            .write(message)
            .await
            .map_err(AsyncConnectionError::WriteError)
    }
}

/// The errors that can occur when using an async connection.
#[derive(Debug, Error)]
pub enum AsyncConnectionError {
    /// An error occurred while wrapping the owned fd for the wayland socket in an AsyncFd.
    #[error("An error occurred while creating the async file descriptor")]
    AsyncFdError(std::io::Error),
    /// An error occurred while writing to the connection.
    #[error("An error occurred while writing to the connection")]
    WriteError(stream::AsyncWriteError),
}
