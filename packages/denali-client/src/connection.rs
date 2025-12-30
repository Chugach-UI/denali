//! A module for establishing and managing a connection to a Wayland server.

use std::{
    collections::HashMap,
    env,
    io::{ErrorKind, IoSlice, IoSliceMut},
    os::{
        fd::{BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd},
        unix::net::UnixStream,
    },
    path::PathBuf,
};

use denali_protocol::wayland::wl_display::WlDisplay;
use thiserror::Error;
use tokio::{
    signal::unix::{Signal, SignalKind, signal},
    sync::mpsc::{self, UnboundedSender},
};
use tokio_seqpacket::{UnixSeqpacket, ancillary::OwnedAncillaryMessage};

use denali_core::{
    connection::ConnectionType,
    handler::Handler,
    id::{AnyObjectId, ObjectId},
    message,
    wire::{
        encode_message,
        serde::{Decode, MessageHeader, RawObjectId, SerdeError},
    },
};

/// A basic, single threaded, implementation of a client connection to a Wayland server.
pub struct Connection<'a> {
    socket: UnixSeqpacket,
    encoding_buf: Vec<u8>,

    handlers: HashMap<RawObjectId, Box<dyn Handler + 'a>>,
}

impl Connection<'_> {
    /// Creates a new Connection to a Wayland server.
    ///
    /// # Errors
    ///
    /// This function will return an error if the XDG runtime directory cannot be located (`XDG_RUNTIME_DIR` environment variable is not set)
    pub fn new() -> Result<(Self, ObjectId<WlDisplay>), ConnectionError> {
        let socket_fd = {
            if let Ok(socket) = env::var("WAYLAND_SOCKET") {
                unsafe { OwnedFd::from_raw_fd(socket.parse().unwrap()) }
            } else {
                let wayland_display = env::var("WAYLAND_DISPLAY").unwrap_or("wayland-0".into());
                let mut wayland_display = PathBuf::from(wayland_display);
                if !wayland_display.is_absolute() {
                    let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR")
                        .map_err(|_| ConnectionError::NoXdgRuntimeDir)?;
                    let xdg_runtime_dir = PathBuf::from(xdg_runtime_dir);
                    wayland_display = xdg_runtime_dir.join(wayland_display);
                }
                unsafe {
                    OwnedFd::from_raw_fd(
                        UnixStream::connect(wayland_display)
                            .map_err(ConnectionError::ConnectError)?
                            .into_raw_fd(),
                    )
                }
            }
        };

        let socket = UnixSeqpacket::try_from(socket_fd).map_err(ConnectionError::ConnectError)?;

        let display_id = unsafe { ObjectId::new(AnyObjectId::new(1)) };

        Ok((
            Self {
                socket,
                encoding_buf: Vec::new(),
                handlers: HashMap::new(),
            },
            display_id,
        ))
    }

    async fn recv_header(&mut self) -> Result<MessageHeader, ConnectionError> {
        let mut header = [0; 8];
        self.socket
            .recv(&mut header)
            .await
            .map_err(ConnectionError::RecvError)?;
        MessageHeader::decode(&header).map_err(ConnectionError::SerdeError)
    }

    /// Receives data along with file descriptors from the Wayland server.
    ///
    /// # Errors
    ///
    /// This function will return an error if receiving the message fails.
    /// See [UnixSeqpacket::recv_vectored_with_ancillary] for more details.
    async fn recv_with_ancillary(
        &self,
        buf: &mut [u8],
        fds: &mut [OwnedFd],
    ) -> Result<usize, ConnectionError> {
        let buffer = IoSliceMut::new(buf);
        let mut ancillary_buffer = [0; 128];
        let (bytes_read, ancillary_reader) = self
            .socket
            .recv_vectored_with_ancillary(&mut [buffer], &mut ancillary_buffer[..])
            .await
            .unwrap();

        for res in ancillary_reader.into_messages() {
            if let OwnedAncillaryMessage::FileDescriptors(received_fds) = res {
                for (dst, src) in fds.iter_mut().zip(received_fds) {
                    *dst = src;
                }
            }
        }

        Ok(bytes_read)
    }

    /// Waits for the next wayland packet
    async fn next_packet(&mut self) -> Result<WaylandPacket, ConnectionError> {
        let head = self.recv_header().await?;

        let size = head.size as usize - 8;
        let mut buf = vec![0u8; size];

        self.recv_with_ancillary(&mut buf, &mut []).await.unwrap();

        Ok(WaylandPacket {
            header: head,
            body: buf,
        })
    }
}

impl<'a> denali_core::connection::Connection<'a> for Connection<'a> {
    fn connection_type(&self) -> ConnectionType {
        ConnectionType::Client
    }

    fn add_handler<I: denali_core::Interface, H: Handler + 'a>(
        &mut self,
        object: &ObjectId<I>,
        handler: H,
    ) {
        self.handlers.insert(object.get(), Box::new(handler));
    }

    fn send_message<
        M: denali_core::message::MessageTypeMarker,
        O: denali_core::message::OutgoingMessage<M>,
    >(
        &mut self,
        message: O,
    ) -> Result<O::Response, ()> {
        const {
            if M::EVENT {
                panic!(
                    "Client sided connections cannot send events over the wire. Make sure to only send requests through this connection."
                )
            }
        }

        // Reserve space for the message in the encoding buffer
        self.encoding_buf
            .reserve(message.size().saturating_sub(self.encoding_buf.capacity()));

        // Encode the message
        encode_message(
            &message,
            message.sender().get(),
            O::OPCODE,
            &mut self.encoding_buf,
        )
        .map_err(|_| ())?;

        // Send the message over the socket
        dbg!(&self.encoding_buf);
        todo!()
    }
}

pub struct WaylandPacket {
    pub header: MessageHeader,
    pub body: Vec<u8>,
}

/// Errors that can occur when establishing a connection to a Wayland server.
#[derive(Debug, Error)]
pub enum ConnectionError {
    /// The `XDG_RUNTIME_DIR` environment variable is not set.
    #[error("XDG_RUNTIME_DIR cannot be found in the environment.")]
    NoXdgRuntimeDir,
    /// Could not connect to the Wayland display.
    #[error("Could not connect to wayland display.")]
    ConnectError(std::io::Error),
    /// Could not clone the underlying Unix stream.
    #[error("Could not clone the stream.")]
    CloneError(std::io::Error),
    /// Could not send the message.
    #[error("Could not send the message.")]
    SendError(std::io::Error),
    /// Could not receive the message.
    #[error("Could not receive the message.")]
    RecvError(std::io::Error),
    /// Error serializing or deserializing the message.
    #[error("Error serializing or deserializing the message.")]
    SerdeError(SerdeError),
}
