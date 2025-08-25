use std::{env, os::unix::net::UnixStream, path::PathBuf};

use thiserror::Error;

use denali_utils::{id_manager::IdManager, proxy::Proxy};

use super::protocol::wayland::wl_display::WlDisplay;

pub struct DisplayConnection {
    socket: UnixStream,
    id_manager: IdManager,
    display: WlDisplay,
}

impl DisplayConnection {
    pub fn new() -> Result<Self, DisplayConnectionError> {
        let wayland_display = env::var_os("WAYLAND_DISPLAY").unwrap_or("wayland-0".into());
        // TODO: cleanup this string processing?
        let socket = Self::connect_to_display(wayland_display.to_string_lossy().into_owned())?;
        let (sender, _receiver) = crossbeam::channel::unbounded();

        let id_manager = IdManager::default();

        let display = WlDisplay::from(Proxy::new(1, id_manager.clone(), sender).unwrap());

        Ok(Self {
            socket,
            id_manager,
            display,
        })
    }

    pub fn display(&self) -> &WlDisplay {
        &self.display
    }

    // TODO: definately clean up this shit
    fn connect_to_display(wayland_display: String) -> Result<UnixStream, DisplayConnectionError> {
        let wayland_display = PathBuf::from(wayland_display);
        let path = if !wayland_display.is_absolute() {
            let xdg_runtime_dir = PathBuf::from(env::var_os("XDG_RUNTIME_DIR").unwrap());
            xdg_runtime_dir.join(wayland_display)
        } else {
            wayland_display
        };

        UnixStream::connect(path).map_err(DisplayConnectionError::ConnectError)
    }
}

#[derive(Debug, Error)]
pub enum DisplayConnectionError {
    #[error("Failed to establish unix socket connection to wayland display server.")]
    ConnectError(#[from] std::io::Error),
}
