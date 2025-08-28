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
        let connection = Connection::new().unwrap();
        let display = WlDisplay::from(
            Proxy::new(1, id_manager.clone(), connection.request_sender()).unwrap(),
        );

        Ok(Self {
            id_manager,
            display,
            connection,
        })
    }

    pub fn display(&self) -> &WlDisplay {
        &self.display
    }

    pub async fn get_event(&self) -> Vec<u8> {
        let head = self.connection.receiver().recv_header().await.unwrap();
        println!("{head:?}");
        let size = head.size as usize - 8;
        let mut buf = vec![0u8; size];
        self.connection
            .receiver()
            .recv_with_ancillary(&mut buf, &mut [])
            .await
            .unwrap();
        buf
    }
}

#[derive(Debug, Error)]
pub enum DisplayConnectionError {
    #[error("Failed to establish unix socket connection to wayland display server.")]
    ConnectError(#[from] std::io::Error),
}
