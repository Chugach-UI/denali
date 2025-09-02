use std::{collections::BTreeMap, rc::Rc, sync::Mutex};

use thiserror::Error;

use denali_core::{
    connection::Connection,
    handler::{Handler, Message},
    id_manager::IdManager,
    proxy::{InterfaceMap, Proxy}, wire::serde::Encode,
};

use super::protocol::wayland::wl_display::WlDisplay;

pub struct DisplayConnection {
    display: WlDisplay,
    connection: Connection,
    interface_map: InterfaceMap,
}

impl DisplayConnection {
    pub fn new() -> Result<Self, DisplayConnectionError> {
        let id_manager = IdManager::default();
        let connection = Connection::new().unwrap();
        let interface_map = Rc::new(Mutex::new(BTreeMap::new()));

        // Pre-insert the wl_display interface into the map with object ID 1
        let init_id = id_manager.peek_next_id().unwrap();
        interface_map
            .lock()
            .unwrap()
            .insert(init_id, "wl_display".to_string());
        let display = WlDisplay::from(
            Proxy::new(
                1, // wl_display version is locked at 1
                id_manager.clone(),
                connection.request_sender(),
                interface_map.clone(),
            )
            .unwrap(),
        );

        Ok(Self {
            display,
            connection,
            interface_map,
        })
    }

    #[must_use]
    pub const fn display(&self) -> &WlDisplay {
        &self.display
    }

    pub async fn handle_event<M: Message + std::fmt::Debug, H: Handler<M>>(&self, handler: &mut H) {
        let head = self.connection.receiver().recv_header().await.unwrap();
        let size = head.size as usize - 8;
        let mut buf = vec![0u8; size];

        self.connection
        .receiver()
        .recv_with_ancillary(&mut buf, &mut [])
        .await
        .unwrap();
    
        let mut head_buf = [0u8; 8];
        head.encode(&mut head_buf).unwrap();

        let map = self.interface_map.lock().unwrap();
        let interface = map.get(&head.object_id);
        if let Some(interface) = interface {
            let msg = M::try_decode(interface, head.opcode, &buf);
            match msg {
                Err(e) => {
                    println!("Failed to decode message for interface {interface:?}: {e}");
                    // println!("Header: {head:?}, Data: {buf:x?}, size: {size}, data_len: {}", buf.len());
                }
                Ok(msg) => handler.handle(msg, head.object_id),
            }
        } else {
            println!("Unhandled message for interface {interface:?}: {head:?}");
        }
    }
}

#[derive(Debug, Error)]
pub enum DisplayConnectionError {
    #[error("Failed to establish unix socket connection to wayland display server.")]
    ConnectError(#[from] std::io::Error),
}
