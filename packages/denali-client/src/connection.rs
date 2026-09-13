//! A module for establishing and managing a connection to a Wayland server.

use std::{
    collections::VecDeque,
    env,
    io::{IoSlice, IoSliceMut},
    os::fd::{AsRawFd, FromRawFd, RawFd},
    os::unix::net::UnixStream,
    path::PathBuf,
};

use async_io::Async;
use denali_protocol_base::wayland::wl_display::{WlDisplay, WlDisplayEvent, WlDisplaySyncRequest};
use nix::{
    cmsg_space,
    sys::socket::{ControlMessage, ControlMessageOwned, MsgFlags, recvmsg, sendmsg},
};
use thiserror::Error;

use denali_core::{
    Interface,
    connection::{ClientConnection, ConnectionType},
    id::{IdFactory, IdManager, IdManagerError, ObjectId},
    message::{DecodeMessageError, Event, MessageResponse, Request, encode_message},
    prelude::IncomingMessage,
    wire::serde::{CompileTimeMessageSize, Decode, MessageHeader, RawObjectId, SerdeError},
};

/// The `wl_display` object always has ID 1.
const DISPLAY_ID: RawObjectId = 1;

fn peek_front_bytes<T: Copy>(queue: &VecDeque<T>, dest: &mut [T]) -> usize {
    let (front, back) = queue.as_slices();
    let front_len = front.len().min(dest.len());
    let back_len = dest.len().saturating_sub(front.len());

    dest[..front_len].copy_from_slice(&front[..front_len]);
    dest[front_len..][..back_len].copy_from_slice(&back[..back_len]);

    front_len + back_len
}

struct WaylandSocket {
    socket: Async<UnixStream>,

    message_buffer: VecDeque<u8>,
    fd_queue: VecDeque<RawFd>,

    cmsg_buffer: Vec<u8>,

    // Bytes to drain from message_buffer at the start of the next operation.
    // This deferred drain allows decoded messages to borrow from the buffer.
    pending_drain: usize,
}
impl WaylandSocket {
    pub fn new(socket: Async<UnixStream>) -> Self {
        Self {
            socket,
            message_buffer: VecDeque::with_capacity(1024),
            fd_queue: VecDeque::with_capacity(10),
            cmsg_buffer: cmsg_space!([RawFd; 10]),
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
        let mut temp_buf = [0; 1024];
        let mut iov = [IoSliceMut::new(&mut temp_buf)];

        let cmsg_buf = &mut self.cmsg_buffer;
        let fd_queue = &mut self.fd_queue;

        let result = self
            .socket
            .read_with(|socket| {
                let fd = socket.as_raw_fd();

                match recvmsg::<()>(fd, &mut iov, Some(cmsg_buf), MsgFlags::empty()) {
                    Ok(msg) => {
                        let mut fds_received = 0;
                        msg.cmsgs()?.into_iter().for_each(|cmsg| {
                            if let ControlMessageOwned::ScmRights(received_fds) = cmsg {
                                fds_received += received_fds.len();
                                fd_queue.extend(received_fds);
                            }
                        });

                        Ok((msg.bytes, fds_received))
                    }
                    Err(e) => Err(std::io::Error::from(e)),
                }
            })
            .await?;

        self.message_buffer.extend(&temp_buf[..result.0]);

        Ok(result)
    }

    /// Writes all of `data` to the socket. File descriptors are sent alongside the first bytes.
    pub async fn send_with_ancillary_data(
        &mut self,
        data: &[u8],
        fds: &[RawFd],
    ) -> std::io::Result<()> {
        let mut sent = 0;

        while sent < data.len() {
            let iov = [IoSlice::new(&data[sent..])];
            let cmsg = if sent == 0 && !fds.is_empty() {
                vec![ControlMessage::ScmRights(fds)]
            } else {
                vec![]
            };

            sent += self
                .socket
                .write_with(|socket| {
                    sendmsg::<()>(socket.as_raw_fd(), &iov, &cmsg, MsgFlags::empty(), None)
                        .map_err(std::io::Error::from)
                })
                .await?;
        }

        Ok(())
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
        self.peek_header_inner().await
    }

    pub async fn skip_message(&mut self) -> Result<(), ConnectionError> {
        let header = self.peek_header().await?;
        self.read_at_least(header.size as _, 0).await?;
        self.message_buffer.drain(..header.size as usize);

        Ok(())
    }

    pub async fn next_message<'a, M: IncomingMessage<'a, Event>>(
        &'a mut self,
        version: u32,
    ) -> Result<M, ConnectionError> {
        self.drain_pending();

        let header = self.peek_header_inner().await?;
        let fd_count = M::fd_count(header.opcode);
        self.read_at_least(header.size as _, fd_count).await?;

        self.message_buffer.make_contiguous();

        //TODO: Remove arbitrary limit of 10 fds to a message (this reasonably should never be exceeded, but it's still gross)
        let mut fd_storage = [0; 10];
        for (slot, fd) in fd_storage.iter_mut().zip(self.fd_queue.drain(..fd_count)) {
            *slot = fd;
        }
        let fds = &fd_storage[..fd_count];

        let (data, _) = self.message_buffer.as_slices();
        let message = M::try_decode(
            header.opcode,
            version,
            &data[MessageHeader::SIZE..header.size as usize],
            fds,
        )?;

        self.pending_drain = header.size as usize;

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
///
/// Events sent to the `wl_display` object are handled internally:
/// `delete_id` recycles the ID and `error` is surfaced as [`ConnectionError::Protocol`].
pub struct Connection {
    socket: WaylandSocket,
    encoding_buf: Vec<u8>,

    id_manager: IdManager,
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
                let fd = socket
                    .parse()
                    .map_err(|_| ConnectionError::InvalidWaylandSocket(socket))?;
                let stream = unsafe { UnixStream::from_raw_fd(fd) };
                Async::new(stream)?
            } else {
                let wayland_display = env::var("WAYLAND_DISPLAY").unwrap_or("wayland-0".into());
                let mut wayland_display = PathBuf::from(wayland_display);
                if !wayland_display.is_absolute() {
                    let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR")
                        .map_err(|_| ConnectionError::NoXdgRuntimeDir)?;
                    let xdg_runtime_dir = PathBuf::from(xdg_runtime_dir);
                    wayland_display = xdg_runtime_dir.join(wayland_display);
                }
                Async::<UnixStream>::connect(wayland_display).await?
            }
        };
        let socket = WaylandSocket::new(socket);

        let display_id = unsafe { id_manager.alloc_typed_id(WlDisplay::MAX_VERSION)? };
        debug_assert_eq!(display_id.get(), DISPLAY_ID);

        Ok((
            Self {
                socket,
                encoding_buf: Vec::new(),
                id_manager,
            },
            display_id,
        ))
    }

    /// Perform a display sync roundtrip, discarding all intermediate events.
    ///
    /// Useful for draining pending events (e.g. `wl_output` info) before
    /// proceeding with further protocol setup.
    pub async fn roundtrip(&mut self, display: &ObjectId<WlDisplay>) -> Result<(), ConnectionError> {
        let sync_cb = self
            .send_request(WlDisplaySyncRequest { sender: display })
            .await?;
        _ = self.recv_event(&sync_cb).await?;
        Ok(())
    }

    /// Peeks the next header, handling any `wl_display` events first.
    async fn peek_header(&mut self) -> Result<MessageHeader, ConnectionError> {
        loop {
            let header = self.socket.peek_header().await?;
            if header.object_id != DISPLAY_ID {
                return Ok(header);
            }

            let event = self
                .socket
                .next_message::<WlDisplayEvent<'_>>(WlDisplay::MAX_VERSION)
                .await?;
            match event {
                WlDisplayEvent::DeleteId { id } => self.id_manager.recycle_id(id),
                WlDisplayEvent::Error {
                    object_id,
                    code,
                    message,
                } => {
                    return Err(ConnectionError::Protocol {
                        object_id: object_id.get(),
                        code,
                        message: message.to_string(),
                    });
                }
            }
        }
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
        let sender = message.sender();
        if O::SINCE > sender.version() {
            return Err(ConnectionError::UnsupportedVersion {
                interface: O::Interface::INTERFACE,
                since: O::SINCE,
                version: sender.version(),
            });
        }
        let new_object_version = message.new_object_version();

        // Reserve space for the message in the encoding buffer
        let required_len = message.size() + MessageHeader::SIZE;
        if self.encoding_buf.len() < required_len {
            self.encoding_buf.resize(required_len, 0);
        }

        //TODO: Remove arbitrary limit
        let mut fd_storage = [0; 10];
        let fds = &mut fd_storage[..O::FD_COUNT];

        // Encode the message
        let len = encode_message(
            &message,
            sender.get(),
            O::OPCODE,
            &mut self.encoding_buf,
            IdFactory::new(&mut self.id_manager),
            fds,
        )?;

        self.socket
            .send_with_ancillary_data(&self.encoding_buf[..len], fds)
            .await?;

        let response = <O::Response as MessageResponse>::with_id_factory(
            IdFactory::new(&mut self.id_manager),
            new_object_version,
        )?;

        Ok(response)
    }

    async fn next_header(&mut self) -> Result<MessageHeader, Self::Error> {
        self.peek_header().await
    }

    async fn decode_message<'a, I: Interface>(
        &'a mut self,
        receiver: &ObjectId<I>,
    ) -> Result<I::Event<'a>, Self::Error> {
        let header = self.peek_header().await?;
        if header.object_id != receiver.get() {
            return Err(ConnectionError::UnexpectedObject {
                expected: receiver.get(),
                actual: header.object_id,
            });
        }

        self.socket.next_message(receiver.version()).await
    }

    async fn skip_message(&mut self) -> Result<(), Self::Error> {
        self.peek_header().await?;
        self.socket.skip_message().await
    }
}

/// Errors that can occur when establishing a connection to a Wayland server.
#[derive(Debug, Error)]
pub enum ConnectionError {
    /// The `XDG_RUNTIME_DIR` environment variable is not set.
    #[error("XDG_RUNTIME_DIR cannot be found in the environment.")]
    NoXdgRuntimeDir,
    /// The `WAYLAND_SOCKET` environment variable is not a file descriptor.
    #[error("WAYLAND_SOCKET is not a valid file descriptor: {0}")]
    InvalidWaylandSocket(String),
    /// IO error occurred.
    #[error("IO error occurred.")]
    Io(#[from] std::io::Error),
    /// Error serializing or deserializing the message.
    #[error("Error serializing or deserializing the message.")]
    SerdeError(#[from] SerdeError),
    /// Error decoding message.
    #[error("Error decoding message.")]
    DecodeError(#[from] DecodeMessageError),
    /// All client IDs have been exhausted.
    #[error("Failed to allocate a new ID.")]
    IdAllocation(#[from] IdManagerError),
    /// A message was sent to an object with a version that does not support it.
    #[error("{interface} version {version} does not support this message (since version {since})")]
    UnsupportedVersion {
        /// The interface of the receiving object.
        interface: &'static str,
        /// The version the message was introduced in.
        since: u32,
        /// The version of the receiving object.
        version: u32,
    },
    /// The next message is addressed to a different object than the one it was decoded for.
    #[error("Expected a message for object {expected}, but the next message is for object {actual}")]
    UnexpectedObject {
        /// The object the message was decoded for.
        expected: RawObjectId,
        /// The object the next message is addressed to.
        actual: RawObjectId,
    },
    /// The server sent a `wl_display.error` event.
    #[error("Protocol error on object {object_id} (code {code}): {message}")]
    Protocol {
        /// The object that caused the error.
        object_id: RawObjectId,
        /// The interface-specific error code.
        code: u32,
        /// A human readable description of the error.
        message: String,
    },
}
