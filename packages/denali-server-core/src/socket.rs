use std::{env, fs, path::{Path, PathBuf}};

use thiserror::Error;
use tokio_seqpacket::UnixSeqpacketListener;

pub struct Socket {
    listener: UnixSeqpacketListener,
}

impl Socket {
    pub fn new() -> Result<Self, SocketError> {
        let xdg_runtime_dir = env::var("XDG_RUNTIME_DIR").map_err(SocketError::NoXdgRuntimeDir)?;
        let mut display_num: usize = 0;
        let path = loop {
            let path = PathBuf::from(&xdg_runtime_dir).join(format!("wayland-{display_num}"));
            if let Ok(path_exists) = fs::exists(&path) && !path_exists {
                break path;
            } 
            display_num += 1;

        };

        Self::new_with_path(&path)
    }

    pub fn new_with_path(path: &Path) -> Result<Self, SocketError> {
        let listener = UnixSeqpacketListener::bind(path).unwrap();
        Ok(Self { listener })
    }
}

#[derive(Debug, Error)]
pub enum SocketError {
    #[error("XDG_RUNTIME_DIR could not be found in the environment")]
    NoXdgRuntimeDir(#[from]env::VarError),
    #[error("Failed to bind socket to address")]
    BindError(#[from]std::io::Error),
}