use std::{collections::BTreeMap, rc::Rc, sync::Mutex};

use thiserror::Error;

use denali_core::{
    handler::{Message, RawHandler},
    id_manager::IdManager,
    store::InterfaceStore,
    wire::serde::{Encode, MessageHeader},
};
use denali_core::{
    proxy::{InterfaceMap, Proxy, SharedProxyState},
    store::Store,
};
use tokio::signal::unix::SignalKind;

use crate::connection::{Connection, ConnectionEvent};

use super::protocol::wayland::wl_display::WlDisplay;

pub struct Event {
    pub header: MessageHeader,
    pub body: Vec<u8>,
}

pub struct DisplayConnection {
    display: WlDisplay,
    connection: Connection,

    shared_state: SharedProxyState,
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
            shared_state: SharedProxyState {
                id_manager,
                request_sender: connection.request_sender(),
                interface_map: interface_map.clone(),
            },
            connection,
        })
    }

    /// Creates a new Store associated with this connection.
    #[must_use]
    pub fn create_store(&self) -> InterfaceStore {
        InterfaceStore::new(self.shared_state.clone())
    }

    #[must_use]
    pub const fn display(&self) -> &WlDisplay {
        &self.display
    }

    pub async fn next_event(&mut self) -> Result<Event, DisplayConnectionError> {
        match self.connection.wait_next_event().await {
           ConnectionEvent::WaylandMessage(head) => {
                let head = head.unwrap();
                let size = head.size as usize - 8;
                let mut buf = vec![0u8; size];

                self.connection
                    .receiver()
                    .recv_with_ancillary(&mut buf, &mut [])
                    .await
                    .unwrap();

                Ok(Event {
                    header: head,
                    body: buf,
                })
            }
            ConnectionEvent::WorkerTerminated(res) => {
                if let Err(e) = res {
                    eprintln!("Worker thread terminated unexpectedly ({e:?})");
                }
                Err(DisplayConnectionError::WorkerTerminated)
            }
            ConnectionEvent::TerminationSignalReceived(
                signal_kind,
            ) => Err(DisplayConnectionError::SignalReceived(signal_kind)),
        }
    }

    pub async fn handle_event<M: Message + std::fmt::Debug, H: RawHandler<M>>(
        &mut self,
        handler: &mut H,
    ) -> Result<(), DisplayConnectionError> {
        let event = self.next_event().await?;

        let map = self.shared_state.interface_map.lock().unwrap();
        let message = map
            .get(&event.header.object_id)
            .map(|iface| M::try_decode(iface, event.header.opcode, &event.body))
            .transpose()
            .map_err(|e| {
                println!(
                    "Failed to decode message for interface {e:?}: {:?}",
                    event.header
                );
                e
            })
            .ok()
            .flatten();

        drop(map);

        if let Some(message) = message {
            handler.handle(message, event.header.object_id);
        } else {
            println!(
                "Unhandled message for interface {message:?}: {:?}",
                event.header
            );
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum DisplayConnectionError {
    #[error("Failed to establish unix socket connection to wayland display server.")]
    ConnectError(#[from] std::io::Error),
    #[error("Connection worker task terminated unexpectedly.")]
    WorkerTerminated,
    #[error("Received SIGHUP, SIGINT, or SIGTERM")]
    SignalReceived(SignalKind),
}
