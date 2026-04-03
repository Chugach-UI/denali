//! Helpers for binding Wayland globals from the registry.

use denali_core::{
    Interface,
    connection::{ClientConnection, Connection as _},
    id::ObjectId,
    message::NewIdHint,
};
use denali_protocol_base::wayland::{
    wl_callback::WlCallbackEvent,
    wl_display::{WlDisplay, WlDisplayGetRegistryRequest, WlDisplaySyncRequest},
    wl_registry::{WlRegistry, WlRegistryBindRequest, WlRegistryEvent},
};
use thiserror::Error;

use crate::connection::{Connection, ConnectionError};

/// A collected global advertisement from the Wayland registry.
#[derive(Debug, Clone)]
pub struct Global {
    pub name: u32,
    pub interface: String,
    pub version: u32,
}

/// The result of the initial registry roundtrip.
///
/// Holds the registry object and a snapshot of all advertised globals,
/// and provides typed `bind` / `try_bind` helpers.
pub struct Registry {
    id: ObjectId<WlRegistry>,
    globals: Vec<Global>,
}

impl Registry {
    /// Perform the initial roundtrip and collect every advertised global.
    pub async fn new(
        conn: &mut Connection,
        display: &ObjectId<WlDisplay>,
    ) -> Result<Self, ConnectionError> {
        let registry = conn
            .send_request(WlDisplayGetRegistryRequest { sender: display })
            .await?;
        let sync_cb = conn
            .send_request(WlDisplaySyncRequest { sender: display })
            .await?;

        let mut globals = Vec::new();

        loop {
            let head = conn.next_header().await?;

            if head.object_id == sync_cb {
                _ = conn.decode_message::<WlCallbackEvent>().await?;
                break;
            }

            if head.object_id == registry {
                let event = conn.decode_message::<WlRegistryEvent<'_>>().await?;

                if let WlRegistryEvent::Global {
                    name,
                    interface,
                    version,
                } = event
                {
                    globals.push(Global {
                        name,
                        interface: interface.as_ref().to_string(),
                        version,
                    });
                }
            }
        }

        Ok(Self {
            id: registry,
            globals,
        })
    }

    /// Bind a required global.
    ///
    /// Returns [`RegistryError::GlobalNotFound`] if the compositor does not
    /// advertise the interface.
    pub async fn bind<I: Interface>(
        &self,
        conn: &mut Connection,
    ) -> Result<ObjectId<I>, RegistryError> {
        self.try_bind::<I>(conn)
            .await?
            .ok_or(RegistryError::GlobalNotFound {
                interface: I::INTERFACE,
            })
    }

    /// Bind an optional global, returning `None` if it is not advertised.
    pub async fn try_bind<I: Interface>(
        &self,
        conn: &mut Connection,
    ) -> Result<Option<ObjectId<I>>, ConnectionError> {
        let Some(global) = self.globals.iter().find(|g| g.interface == I::INTERFACE) else {
            return Ok(None);
        };

        let version = global.version.min(I::MAX_VERSION);

        let id = conn
            .send_request(WlRegistryBindRequest {
                sender: &self.id,
                name: global.name,
                id: NewIdHint::<I>::new(version),
            })
            .await?;

        Ok(Some(id))
    }

    /// The raw list of advertised globals.
    pub fn globals(&self) -> &[Global] {
        &self.globals
    }
}

/// Errors that can occur when binding a global.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// The requested global is not advertised by the compositor.
    #[error("global `{interface}` is not available")]
    GlobalNotFound { interface: &'static str },
    /// An underlying connection error.
    #[error(transparent)]
    Connection(#[from] ConnectionError),
}
