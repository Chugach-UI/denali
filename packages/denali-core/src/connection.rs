use std::{
    env,
    io::{ErrorKind, IoSlice, IoSliceMut},
    os::fd::{BorrowedFd, IntoRawFd, OwnedFd, RawFd},
    path::PathBuf,
};

use thiserror::Error;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio_seqpacket::{
    UnixSeqpacket,
    ancillary::{AddControlMessageError, AncillaryMessageWriter, OwnedAncillaryMessage},
};

use crate::{
    proxy::RequestMessage,
    wire::serde::{Decode, MessageHeader, SerdeError},
};

/// A connection to a Wayland server.
pub struct Connection {
    recv: RecvSocket,
    mpsc_send: mpsc::UnboundedSender<RequestMessage>,
    worker_handle: tokio::task::JoinHandle<()>,
}

impl Connection {
    /// Creates a new Connection to a Wayland server.
    ///
    /// # Errors
    ///
    /// This function will return an error if the XDG runtime directory cannot be located (`XDG_RUNTIME_DIR` environment variable is not set)
    pub async fn new() -> Result<Self, ConnectionError> {
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

        let (mpsc_send, mut mpsc_recv) = mpsc::unbounded_channel::<RequestMessage>();

        let worker_handle = tokio::task::spawn(async move {
            while let Some(msg) = mpsc_recv.recv().await {
                send.send_with_ancillary(msg.buffer.as_slice(), msg.fds.as_slice())
                    .await
                    .unwrap();

                println!("Sent {:?}", msg);
            }

            println!("Worker exiting...");
        });

        Ok(Self {
            recv,
            mpsc_send,
            worker_handle,
        })
    }

    pub fn mpsc_sender(&self) -> UnboundedSender<RequestMessage> {
        self.mpsc_send.clone()
    }

    pub fn receiver(&self) -> &RecvSocket {
        &self.recv
    }
}

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("XDG_RUNTIME_DIR cannot be found in the environment.")]
    NoXdgRuntimeDir,
    #[error("Could not connect to wayland display.")]
    ConnectError(std::io::Error),
    #[error("Could not clone the stream.")]
    CloneError(std::io::Error),
}

struct SendSocket(UnixSeqpacket);

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
enum SendSocketError {
    #[error("Failed to add fds to ancillary buffer")]
    AddFdsFailed(#[from] AddControlMessageError),
    #[error("IO operation failed.")]
    IoError(#[from] std::io::Error),
}

pub struct RecvSocket(pub UnixSeqpacket);

impl RecvSocket {
    pub async fn read(&self, buf: &mut [u8]) {
        self.0.recv(buf).await.unwrap();
    }

    pub async fn recv_header(&self) -> Result<MessageHeader, RecvSocketError> {
        let mut buf = [0u8; 8];
        self.0
            .recv(&mut buf)
            .await
            .map_err(RecvSocketError::IoError)?;
        Ok(MessageHeader::decode(&buf).map_err(RecvSocketError::DecodeHeaderError)?)
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
