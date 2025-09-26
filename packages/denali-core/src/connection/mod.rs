//! A connection to a Wayland server.

use std::{os::fd::OwnedFd, sync::Arc};

use parking_lot::Mutex;
use thiserror::Error;

mod stream;

/// A thread-safe wrapper around a Wayland connection.
pub struct Connection(Arc<Mutex<ConnectionInner>>);

impl Connection {
    /// Create a new connection to a Wayland server using a preestablished socket.
    pub fn new(socket: OwnedFd) -> Result<Self, ConnectionError> {
        let socket_dup = socket
            .try_clone()
            .map_err(ConnectionError::SocketCloneError)?;

        let read_stream = stream::ReadStream::new(socket_dup);
        let write_stream = stream::WriteStream::new(socket);

        Self::start_read_worker(read_stream)?;

        let inner = ConnectionInner { write_stream };

        Ok(Self(Arc::new(Mutex::new(inner))))
    }

    /// Send a message without ancillary to the wayland server.
    pub fn send_message(&self, buf: &[u8]) -> Result<(), stream::WriteError> {
        self.0.lock().write(buf)
    }

    fn start_read_worker(_read_stream: stream::ReadStream) -> Result<(), ConnectionError> {
        Ok(())
    }
}

struct ConnectionInner {
    write_stream: stream::WriteStream,
}

impl ConnectionInner {
    // TEMP
    pub fn write(&self, buf: &[u8]) -> Result<(), stream::WriteError> {
        self.write_stream.write(buf)
    }
}

/// Describes the various errors that can occur when creating or using a connection.
#[derive(Debug, Error)]
pub enum ConnectionError {
    /// Failed to clone the provided socket so that it can be split.
    #[error("Failed to clone socket: {0}")]
    SocketCloneError(std::io::Error),
}
