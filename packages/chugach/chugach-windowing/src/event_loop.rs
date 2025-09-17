use denali_client::core::Object;
use denali_client::core::handler::Message;
use denali_client::{
    core::{
        handler::{Handler, HasStore, RawHandler},
        store::{InterfaceStore, Store},
        wire::serde::ObjectId,
    },
    display_connection::DisplayConnection,
    protocol::wayland::{
        wl_callback::{WlCallback, WlCallbackEvent},
        wl_registry::WlRegistryEvent,
    },
};
use frunk::Coprod;
use log::info;

pub struct CallbackHandler {
    sync_id: ObjectId,
    completed: bool,
}
impl RawHandler<WlCallbackEvent> for CallbackHandler {
    fn handle(&mut self, _message: WlCallbackEvent, object_id: ObjectId) {
        if object_id == self.sync_id {
            self.completed = true;
        }
    }
}
impl CallbackHandler {
    pub fn new(sync: &WlCallback) -> Self {
        Self {
            sync_id: sync.id(),
            completed: false,
        }
    }
}

#[derive(Debug)]
struct EventLoopInner {
    connection: DisplayConnection,
    global_store: InterfaceStore,
}
impl HasStore for EventLoopInner {
    fn store(&self) -> &impl Store {
        &self.global_store
    }

    fn store_mut(&mut self) -> &mut impl Store {
        &mut self.global_store
    }
}
impl Handler<WlRegistryEvent<'_>> for EventLoopInner {
    fn handle(
        &mut self,
        message: WlRegistryEvent<'_>,
        interface: &<WlRegistryEvent<'_> as denali_client::core::handler::MessageTarget>::Target,
    ) {
        match message {
            WlRegistryEvent::Global(ev) => {
                let obj = interface
                    .bind_raw(&ev.interface.data, ev.name, ev.version)
                    .unwrap();
                self.global_store
                    .insert_proxy(ev.interface.data.to_string(), obj.version(), obj);
            }
            WlRegistryEvent::GlobalRemove(ev) => {
                info!("Removed global: {}", ev.name);
            }
        }
    }
}

type EventLoopInnerEvent<'a> = Coprod!(WlRegistryEvent<'a>);

impl EventLoopInner {
    pub async fn handle_all_events(&mut self) -> Result<(), EventLoopError> {
        let sync_cb = self.connection.display().sync();
        let mut sync_handler = CallbackHandler::new(&sync_cb);

        while !sync_handler.completed {
            let event = self.connection.next_event().await?;

            // First handle any sync callback events
            if let Ok(decoded) =
                WlCallbackEvent::try_decode(&event.interface, event.header.opcode, &event.body)
            {
                sync_handler.handle(decoded, event.header.object_id);
            }

            // Then handle other events
            if let Ok(decoded) =
                EventLoopInnerEvent::try_decode(&event.interface, event.header.opcode, &event.body)
            {
                <Self as RawHandler<EventLoopInnerEvent<'_>>>::handle(
                    self,
                    decoded,
                    event.header.object_id,
                );
            } else {
                info!(
                    "Unknown event for interface {}: opcode {}",
                    event.interface, event.header.opcode
                );
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct EventLoop(EventLoopInner);
impl EventLoop {
    pub fn new() -> Result<Self, EventLoopError> {
        let connection = DisplayConnection::new()?;
        let mut store = connection.create_store();

        let disp = connection.display();
        let reg = disp.registry();
        store.insert_interface(reg, 1);

        Ok(Self(EventLoopInner {
            connection,
            global_store: store,
        }))
    }

    pub async fn create_window(&mut self) {
        self.0.handle_all_events();
    }
}

#[derive(thiserror::Error, Debug)]
pub enum EventLoopError {
    #[error("Failed to create display connection")]
    ConnectionError(#[from] denali_client::display_connection::DisplayConnectionError),
}
