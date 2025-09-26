use std::os::fd::{AsRawFd, BorrowedFd, OwnedFd};

use thiserror::Error;

pub struct ReadStream(OwnedFd);

impl ReadStream {
    pub fn new(socket: OwnedFd) -> Self {
        Self(socket)
    }
}

pub struct WriteStream(OwnedFd);

impl WriteStream {
    pub fn new(socket: OwnedFd) -> Self {
        Self(socket)
    }

    pub fn write(&self, data: &[u8]) -> Result<(), WriteError> {
        if unsafe {
            libc::write(
                self.0.as_raw_fd(),
                data.as_ptr() as *const libc::c_void,
                data.len() as libc::size_t,
            )
        } < 0
        {
            return Err(WriteError::IoError(std::io::Error::last_os_error()));
        }

        Ok(())
    }

    pub fn write_with_fds(&self, data: &[u8], fds: &[BorrowedFd]) -> Result<(), WriteError> {
        // TODO: sendmsg hell
        Ok(())
    }
}

/// An error that can occur when writing to the wayland socket.
#[derive(Debug, Error)]
pub enum WriteError {
    /// TODO: make this more descriptive
    #[error("I/O error occurred")]
    IoError(#[from] std::io::Error),
}
