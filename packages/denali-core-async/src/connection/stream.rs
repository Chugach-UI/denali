use std::os::fd::OwnedFd;

use thiserror::Error;
use tokio::io::unix::AsyncFd;

pub struct AsyncWriteStream(AsyncFd<OwnedFd>);

impl AsyncWriteStream {
    pub async fn write(&self, buf: &[u8]) -> Result<(), AsyncWriteError> {
        todo!()
    }
}

#[derive(Debug, Error)]
pub enum AsyncWriteError {
    /// TODO: make this error more specific
    #[error("I/O error")]
    IoError(#[from] std::io::Error),
}
