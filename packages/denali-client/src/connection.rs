//! A module for establishing and managing a connection to a Wayland server.

use std::{
    collections::VecDeque,
    env,
    io::{ErrorKind, IoSlice, IoSliceMut},
    os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd, RawFd},
    path::PathBuf,
};

use denali_protocol_base::wayland::wl_display::WlDisplay;
use nix::{
    cmsg_space,
    sys::socket::{ControlMessage, ControlMessageOwned, MsgFlags, recvmsg, sendmsg},
};
use thiserror::Error;
use tokio::{
    io::{AsyncWriteExt, Interest},
    net::UnixStream,
};

use denali_core::{
    Interface,
    connection::ConnectionType,
    id::{IdFactory, IdManager, ObjectId},
    message::{DecodeMessageError, Event, MessageResponse, Request, encode_message},
    prelude::IncomingMessage,
    wire::serde::{CompileTimeMessageSize, Decode, MessageHeader, SerdeError},
};

fn peek_front_bytes<T: Copy>(queue: &VecDeque<T>, dest: &mut [T]) -> usize {
    let (front, back) = queue.as_slices();
    let front_len = front.len().min(dest.len());
    let back_len = dest.len().saturating_sub(front.len());

    dest[..front_len].copy_from_slice(&front[..front_len]);
    dest[front_len..][..back_len].copy_from_slice(&back[..back_len]);

    front_len + back_len
}

struct WaylandSocket {
    socket: UnixStream,

    message_buffer: VecDeque<u8>,
    fd_queue: VecDeque<RawFd>,

    cmsg_buffer: Vec<u8>,

    // Effectively a numeric id for the current header
    header_counter: u32,
    // Bytes to drain from message_buffer at the start of the next operation.
    // This deferred drain allows decoded messages to borrow from the buffer.
    pending_drain: usize,
}
impl WaylandSocket {
    pub fn new(socket: UnixStream) -> Self {
        Self {
            socket,
            message_buffer: VecDeque::with_capacity(1024),
            fd_queue: VecDeque::with_capacity(10),
            cmsg_buffer: cmsg_space!([RawFd; 10]),
            header_counter: 0,
            pending_drain: 0,
        }
    }

    /// Drains any bytes left over from a previously decoded message.
    fn drain_pending(&mut self) {
        if self.pending_drain > 0 {
            self.message_buffer.drain(..self.pending_drain);
            self.pending_drain = 0;
        }
    }

    /// Reads data from the socket and stores it in the internal buffer to be decoded later.
    /// Returns the number of bytes read and the number of file descriptors received.
    pub async fn read_with_ancillary_data(&mut self) -> std::io::Result<(usize, usize)> {
        self.socket.ready(Interest::READABLE).await?;

        let mut temp_buf = [0; 1024];
        let mut iov = [IoSliceMut::new(&mut temp_buf)];

        let result = self.socket.try_io(Interest::READABLE, || {
            let fd = self.socket.as_raw_fd();

            match recvmsg::<()>(fd, &mut iov, Some(&mut self.cmsg_buffer), MsgFlags::empty()) {
                Ok(msg) => {
                    let mut fds_received = 0;
                    msg.cmsgs()?.into_iter().for_each(|cmsg| {
                        if let ControlMessageOwned::ScmRights(received_fds) = cmsg {
                            fds_received += received_fds.len();
                            self.fd_queue.extend(received_fds.into_iter());
                        }
                    });

                    Ok((msg.bytes, fds_received))
                }
                Err(e) => Err(std::io::Error::from(e)),
            }
        })?;

        self.message_buffer.extend(&temp_buf[..result.0]);

        Ok(result)
    }

    pub async fn send_with_ancillary_data(
        &mut self,
        data: &[u8],
        fds: &[RawFd],
    ) -> std::io::Result<usize> {
        self.socket.ready(Interest::WRITABLE).await?;

        let iov = [IoSlice::new(data)];

        let result = self.socket.try_io(Interest::WRITABLE, || {
            let socket_fd = self.socket.as_raw_fd();

            let cmsg = if !fds.is_empty() {
                vec![ControlMessage::ScmRights(fds)]
            } else {
                vec![]
            };

            match sendmsg::<()>(socket_fd, &iov, &cmsg, MsgFlags::empty(), None) {
                Ok(bytes_sent) => Ok(bytes_sent),
                Err(e) => Err(std::io::Error::from(e)),
            }
        })?;

        Ok(result)
    }

    pub async fn read_at_least(&mut self, bytes: usize, fds: usize) -> Result<(), std::io::Error> {
        while self.message_buffer.len() < bytes || self.fd_queue.len() < fds {
            match self.read_with_ancillary_data().await {
                Ok(_) => {}
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    pub async fn peek_header(&mut self) -> Result<MessageHeader, ConnectionError> {
        self.drain_pending();

        let mut data = [0u8; MessageHeader::SIZE];

        self.read_at_least(MessageHeader::SIZE, 0).await?;
        peek_front_bytes(&self.message_buffer, &mut data);

        MessageHeader::decode(&data).map_err(Into::into)
    }

    pub async fn flush_next_message(&mut self) -> Result<(), ConnectionError> {
        let header = self.peek_header().await?;
        self.read_at_least(header.size as _, 0).await?;
        self.message_buffer.drain(..header.size as usize);
        self.header_counter += 1;

        Ok(())
    }

    pub async fn next_message<'a, M: IncomingMessage<'a, Event>>(&'a mut self) -> Result<M, ConnectionError> {
        self.drain_pending();

        let header = self.peek_header_inner().await?;
        self.read_at_least(header.size as _, M::fd_count(header.opcode))
            .await?;

        self.message_buffer.make_contiguous();

        //TODO: Remove arbitrary limit of 10 fds to a message (this reasonably should never be exceeded, but it's still gross)
        let fd_storage = [0; 10];
        let fds = &fd_storage[..M::fd_count(header.opcode)];

        let (data, _) = self.message_buffer.as_slices();
        let message = M::try_decode(
            M::Interface::INTERFACE,
            header.opcode,
            denali_core::message::MessageType::Event,
            &data[MessageHeader::SIZE..],
            &fds,
        )?;

        self.pending_drain = header.size as usize;
        self.header_counter += 1;

        Ok(message)
    }

    /// Peeks the header without draining pending data first.
    /// Used internally by `next_message` which drains at the start.
    async fn peek_header_inner(&mut self) -> Result<MessageHeader, ConnectionError> {
        let mut data = [0u8; MessageHeader::SIZE];

        self.read_at_least(MessageHeader::SIZE, 0).await?;
        peek_front_bytes(&self.message_buffer, &mut data);

        MessageHeader::decode(&data).map_err(Into::into)
    }
}

/// A basic, single threaded, implementation of a client connection to a Wayland server.
pub struct Connection {
    socket: WaylandSocket,
    encoding_buf: Vec<u8>,

    id_manager: IdManager,

    last_peeked_header: Option<u32>,
}

impl Connection {
    /// Creates a new Connection to a Wayland server.
    ///
    /// # Errors
    ///
    /// This function will return an error if the XDG runtime directory cannot be located (`XDG_RUNTIME_DIR` environment variable is not set)
    pub async fn new() -> Result<(Self, ObjectId<WlDisplay>), ConnectionError> {
        let mut id_manager = IdManager::new();

        let socket = {
            if let Ok(socket) = env::var("WAYLAND_SOCKET") {
                let stream =
                    unsafe { std::os::unix::net::UnixStream::from_raw_fd(socket.parse().unwrap()) };
                stream.set_nonblocking(true).unwrap();
                UnixStream::from_std(stream)?
            } else {
                let wayland_display = env::var("WAYLAND_DISPLAY").unwrap_or("wayland-0".into());
                let mut wayland_display = PathBuf::from(wayland_display);
                if !wayland_display.is_absolute() {
                    let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR")
                        .map_err(|_| ConnectionError::NoXdgRuntimeDir)?;
                    let xdg_runtime_dir = PathBuf::from(xdg_runtime_dir);
                    wayland_display = xdg_runtime_dir.join(wayland_display);
                }
                UnixStream::connect(wayland_display).await?
            }
        };
        let socket = WaylandSocket::new(socket);

        let display_id = unsafe { id_manager.alloc_typed_id().unwrap() };

        Ok((
            Self {
                socket,
                encoding_buf: Vec::new(),
                id_manager,
                last_peeked_header: None,
            },
            display_id,
        ))
    }
}

impl denali_core::connection::Connection for Connection {
    type Error = ConnectionError;
    type IncomingMessageType = Event;

    fn connection_type(&self) -> ConnectionType {
        ConnectionType::Client
    }

    async fn send_message<O: denali_core::message::OutgoingMessage<Request>>(
        &mut self,
        message: O,
    ) -> Result<O::Response, ConnectionError> {
        // Reserve space for the message in the encoding buffer
        let required_len = self.encoding_buf.len() + message.size() + MessageHeader::SIZE;
        let growth = required_len.saturating_sub(self.encoding_buf.capacity());
        self.encoding_buf
            .resize(self.encoding_buf.len() + growth, 0);

        //TODO: Remove arbitrary limit
        let mut fd_storage = [0; 10];
        let fds = &mut fd_storage[..O::FD_COUNT];

        // Encode the message
        let len = encode_message(
            &message,
            message.sender().get(),
            O::OPCODE,
            &mut self.encoding_buf,
            IdFactory::new(&mut self.id_manager),
            fds,
        )?;

        self.socket
            .send_with_ancillary_data(&self.encoding_buf[..len], &fds)
            .await?;
        self.socket.socket.flush().await?;

        let response =
            <O::Response as MessageResponse>::with_id_factory(IdFactory::new(&mut self.id_manager));

        Ok(response)
    }

    async fn next_header(&mut self) -> Result<MessageHeader, Self::Error> {
        let header = self.socket.peek_header().await?;

        let Some(last_header) = self.last_peeked_header else {
            self.last_peeked_header = Some(self.socket.header_counter);
            return Ok(header);
        };

        // Check if we are peeking the same header again
        if last_header == self.socket.header_counter {
            self.socket.flush_next_message().await?;
            self.last_peeked_header = None;
            return self.socket.peek_header().await;
        } else {
            self.last_peeked_header = Some(self.socket.header_counter);
        }

        Ok(header)
    }
    async fn decode_message<'a, M: IncomingMessage<'a, Self::IncomingMessageType>>(
        &'a mut self,
    ) -> Result<M, Self::Error> {
        let message = self.socket.next_message::<M>().await?;

        self.last_peeked_header = None;

        Ok(message)
    }
}

/// Errors that can occur when establishing a connection to a Wayland server.
#[derive(Debug, Error)]
pub enum ConnectionError {
    /// The `XDG_RUNTIME_DIR` environment variable is not set.
    #[error("XDG_RUNTIME_DIR cannot be found in the environment.")]
    NoXdgRuntimeDir,
    /// IO error occurred.
    #[error("IO error occurred.")]
    Io(#[from] std::io::Error),
    /// Error serializing or deserializing the message.
    #[error("Error serializing or deserializing the message.")]
    SerdeError(#[from] SerdeError),
    /// Error decoding message.
    #[error("Error decoding message.")]
    DecodeError(#[from] DecodeMessageError),
}
