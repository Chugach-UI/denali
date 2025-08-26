use thiserror::Error;

use denali_core::{connection::Connection, id_manager::IdManager, proxy::Proxy};

use super::protocol::wayland::wl_display::WlDisplay;

pub struct DisplayConnection {
    id_manager: IdManager,
    display: WlDisplay,
    connection: Connection,
}

impl DisplayConnection {
    pub async fn new() -> Result<Self, DisplayConnectionError> {
        let id_manager = IdManager::default();
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        let display = WlDisplay::from(Proxy::new(1, id_manager.clone(), sender).unwrap());

        let connection = Connection::new().await.unwrap();

        Ok(Self {
            id_manager,
            display,
            connection,
        })
    }

    pub fn display(&self) -> &WlDisplay {
        &self.display
    }
}

#[derive(Debug, Error)]
pub enum DisplayConnectionError {
    #[error("Failed to establish unix socket connection to wayland display server.")]
    ConnectError(#[from] std::io::Error),
}
