//! A module for establishing and managing a connection to a Wayland server.

use std::{
    env,
    io::{ErrorKind, IoSlice, IoSliceMut},
    os::fd::{BorrowedFd, IntoRawFd, OwnedFd, RawFd},
    path::PathBuf,
};

use thiserror::Error;
use tokio::{
    select, signal::unix::{signal, Signal, SignalKind}, sync::mpsc::{self, UnboundedSender}
};
use tokio_seqpacket::{
    UnixSeqpacket,
    ancillary::{AddControlMessageError, AncillaryMessageWriter, OwnedAncillaryMessage},
};
use tracing::error;

use denali_core::proxy::RequestMessage;
use denali_core::wire::serde::{Decode, MessageHeader, SerdeError};
use tokio_util::sync::CancellationToken;

/// A connection to a Wayland server.
#[derive(Debug)]
pub struct Connection {
    recv: RecvSocket,
    request_sender: mpsc::UnboundedSender<RequestMessage>,
    worker_handle: tokio::task::JoinHandle<Result<(), SendSocketError>>,
    cancel_token: CancellationToken,
}

impl Connection {
    /// Creates a new Connection to a Wayland server.
    ///
    /// # Errors
    ///
    /// This function will return an error if the XDG runtime directory cannot be located (`XDG_RUNTIME_DIR` environment variable is not set)
    pub fn new() -> Result<Self, ConnectionError> {
        let wayland_display = env::var("WAYLAND_DISPLAY").unwrap_or("wayland-0".to_string());
        let mut wayland_display = PathBuf::from(wayland_display);
        if !wayland_display.is_absolute() {
            let xdg_runtime_dir =
                env::var("XDG_RUNTIME_DIR").map_err(|_| ConnectionError::NoXdgRuntimeDir)?;
            let xdg_runtime_dir = PathBuf::from(xdg_runtime_dir);
            wayland_display = xdg_runtime_dir.join(wayland_display);
        }

        let stream = std::os::unix::net::UnixStream::connect(wayland_display)
            .map_err(ConnectionError::ConnectError)?;

        let stream_dup = stream.try_clone().map_err(ConnectionError::CloneError)?;

        let send: SendSocket;
        let recv: RecvSocket;
        unsafe {
            send = UnixSeqpacket::from_raw_fd(stream.into_raw_fd())
                .unwrap()
                .into();
            recv = UnixSeqpacket::from_raw_fd(stream_dup.into_raw_fd())
                .unwrap()
                .into();
        }

        let (request_sender, mut request_receiver) = mpsc::unbounded_channel::<RequestMessage>();

        let token = CancellationToken::new();
        let worker_token = token.clone();

        let worker_handle = tokio::task::spawn(async move {
            loop {
                select! {
                    () = worker_token.cancelled() => {
                        break;
                    }
                    res = request_receiver.recv() => {
                        match res {
                            Some(msg) => {
                                if let Err(e) = send.send_with_ancillary(msg.buffer.as_slice(), msg.fds.as_slice()).await {
                                    error!("Failed to send message to Wayland server: {e}");
                                    return Err(e);
                                }
                            }
                            None => break, // Channel closed, exit the loop
                        }
                    }
                }
            }
            Ok(())
        });

        Ok(Self {
            recv,
            request_sender,
            worker_handle,
            cancel_token: token,
        })
    }

    /// Returns a sender that can be used to send requests to the Wayland server.
    #[must_use]
    pub fn request_sender(&self) -> UnboundedSender<RequestMessage> {
        self.request_sender.clone()
    }

    /// Returns a reference to the receiver socket.
    #[must_use]
    pub const fn receiver(&self) -> &RecvSocket {
        &self.recv
    }

    /// Waits for the next async event to occur, which can either be a wayland packet, a worker thread failure, or a unix signal
    pub async fn wait_next_event(&mut self) -> ConnectionEvent {
        tokio::select! {
            head = self.recv.recv_header() => {
                ConnectionEvent::WaylandMessage(head)
            },
            Ok(res) = &mut self.worker_handle => {
                error!("Worker task terminated.");
                ConnectionEvent::WorkerTerminated(res)
            },
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}

pub enum ConnectionEvent {
    WaylandMessage(Result<MessageHeader, RecvSocketError>),
    WorkerTerminated(Result<(), SendSocketError>),
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
}

pub struct SendSocket(UnixSeqpacket);

impl SendSocket {
    /// Sends data along with file descriptors to the Wayland server.
    ///
    /// # Errors
    ///
    /// This function will return an error if sending the message fails.
    /// See [UnixSeqpacket::send_vectored_with_ancillary] for more details.
    pub async fn send_with_ancillary(
        &self,
        buf: &[u8],
        fds: &[RawFd],
    ) -> Result<(), SendSocketError> {
        let buffer = IoSlice::new(buf);
        let mut ancillary_buffer = [0; 128];
        let mut ancillary = AncillaryMessageWriter::new(&mut ancillary_buffer[..]);
        let fds = fds
            .iter()
            .map(|fd| unsafe { BorrowedFd::borrow_raw(*fd) })
            .collect::<Vec<_>>();

        ancillary
            .add_fds(&fds)
            .map_err(SendSocketError::AddFdsFailed)?;

        while let Err(err) = self
            .0
            .send_vectored_with_ancillary(&[buffer], &mut ancillary)
            .await
        {
            match err.kind() {
                ErrorKind::Interrupted => {}
                _ => return Err(SendSocketError::IoError(err)),
            }
        }

        Ok(())
    }
}

impl From<UnixSeqpacket> for SendSocket {
    fn from(value: UnixSeqpacket) -> Self {
        Self(value)
    }
}

#[derive(Debug, Error)]
pub enum SendSocketError {
    #[error("Failed to add fds to ancillary buffer")]
    AddFdsFailed(#[from] AddControlMessageError),
    #[error("IO operation failed.")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct RecvSocket(UnixSeqpacket);

impl RecvSocket {
    pub async fn recv(&self, buf: &mut [u8]) {
        self.0.recv(buf).await.unwrap();
    }

    pub async fn recv_header(&self) -> Result<MessageHeader, RecvSocketError> {
        let mut buf = [0u8; 8];
        self.0
            .recv(&mut buf)
            .await
            .map_err(RecvSocketError::IoError)?;
        MessageHeader::decode(&buf).map_err(RecvSocketError::DecodeHeaderError)
    }

    /// Receives data along with file descriptors from the Wayland server.
    ///
    /// # Errors
    ///
    /// This function will return an error if receiving the message fails.
    /// See [UnixSeqpacket::recv_vectored_with_ancillary] for more details.
    pub async fn recv_with_ancillary(
        &self,
        buf: &mut [u8],
        fds: &mut [OwnedFd],
    ) -> Result<usize, ConnectionError> {
        let buffer = IoSliceMut::new(buf);
        let mut ancillary_buffer = [0; 128];
        let (bytes_read, ancillary_reader) = self
            .0
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
}

impl From<UnixSeqpacket> for RecvSocket {
    fn from(value: UnixSeqpacket) -> Self {
        Self(value)
    }
}

#[derive(Debug, Error)]
pub enum RecvSocketError {
    #[error("Failed to decode header buffer.")]
    DecodeHeaderError(#[from] SerdeError),
    #[error("IO operation failed.")]
    IoError(#[from] std::io::Error),
}
