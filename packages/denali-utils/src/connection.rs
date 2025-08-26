use std::{
    env,
    io::{ErrorKind, IoSlice, IoSliceMut},
    os::fd::{BorrowedFd, OwnedFd},
    path::PathBuf,
};
use thiserror::Error;
use tokio_seqpacket::{
    UnixSeqpacket,
    ancillary::{AncillaryMessageWriter, OwnedAncillaryMessage},
};

pub struct Connection {
    stream: UnixSeqpacket,
}

impl Connection {
    pub async fn new() -> Result<Self, ConnectionError> {
        let wayland_display = env::var("WAYLAND_DISPLAY").unwrap_or("wayland-0".to_string());
        let mut wayland_display = PathBuf::from(wayland_display);
        if !wayland_display.is_absolute() {
            let xdg_runtime_dir =
                env::var("XDG_RUNTIME_DIR").map_err(|_| ConnectionError::NoXdgRuntimeDir)?;
            let xdg_runtime_dir = PathBuf::from(xdg_runtime_dir);
            wayland_display = xdg_runtime_dir.join(wayland_display);
        }

        let stream = UnixSeqpacket::connect(wayland_display)
            .await
            .map_err(ConnectionError::ConnectError)?;

        Ok(Self { stream })
    }

    pub async fn send_with_ancillary<'a>(
        &self,
        buf: &[u8],
        fds: &[BorrowedFd<'a>],
    ) -> Result<(), ConnectionError> {
        let buffer = IoSlice::new(buf);
        let mut ancillary_buffer = [0; 128];
        let mut ancillary = AncillaryMessageWriter::new(&mut ancillary_buffer[..]);
        ancillary.add_fds(fds).unwrap();

        while let Err(err) = self
            .stream
            .send_vectored_with_ancillary(&[buffer], &mut ancillary)
            .await
        {
            match err.kind() {
                ErrorKind::Interrupted => continue,
                _ => return Err(ConnectionError::SendError(err)),
            }
        }

        Ok(())
    }

    pub async fn recv_with_ancillary(
        &self,
        buf: &mut [u8],
        fds: &mut [OwnedFd],
    ) -> Result<usize, ConnectionError> {
        let buffer = IoSliceMut::new(buf);
        let mut ancillary_buffer = [0; 128];
        let (bytes_read, ancillary_reader) = self
            .stream
            .recv_vectored_with_ancillary(&mut [buffer], &mut ancillary_buffer[..])
            .await
            .unwrap();

        for res in ancillary_reader.into_messages() {
            if let OwnedAncillaryMessage::FileDescriptors(received_fds) = res {
                for (dst, src) in fds.iter_mut().zip(received_fds) {
                    *dst = src
                }
            }
        }

        Ok(bytes_read)
    }
}

#[derive(Debug, Error)]
pub enum ConnectionError {
    #[error("XDG_RUNTIME_DIR cannot be found in the environment.")]
    NoXdgRuntimeDir,
    #[error("Could not connect to wayland display.")]
    ConnectError(std::io::Error),
    #[error("Failed to sendmsg.")]
    SendError(std::io::Error),
}
