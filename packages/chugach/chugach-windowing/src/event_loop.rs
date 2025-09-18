use denali_client::core::Object;
use denali_client::core::handler::Message;
use denali_client::protocol::wayland::wl_compositor::WlCompositor;
use denali_client::protocol::xdg_shell::xdg_wm_base::XdgWmBase;
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
    store: InterfaceStore,

    compositor: Option<ObjectId>,
    xdg_wm_base: Option<ObjectId>,
}
impl HasStore for EventLoopInner {
    fn store(&self) -> &impl Store {
        &self.store
    }

    fn store_mut(&mut self) -> &mut impl Store {
        &mut self.store
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
                self.store
                    .insert_proxy(ev.interface.data.to_string(), obj.version(), obj);
            }
            WlRegistryEvent::GlobalRemove(ev) => {
                tracing::info!("Removed global: {}", ev.name);
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
            tracing::debug!(iface = ?event.interface);
            let Some(interface) = event.interface.as_deref() else {
                tracing::warn!("No interface for object {}", event.header.object_id);
                continue;
            };

            // First handle any sync callback events
            if event.header.object_id == sync_handler.sync_id
                && let Ok(decoded) =
                    WlCallbackEvent::try_decode(interface, event.header.opcode, &event.body)
            {
                sync_handler.handle(decoded, event.header.object_id);
            }
            // Then handle other events
            else if let Ok(decoded) =
                EventLoopInnerEvent::try_decode(interface, event.header.opcode, &event.body)
            {
                <Self as RawHandler<EventLoopInnerEvent<'_>>>::handle(
                    self,
                    decoded,
                    event.header.object_id,
                );
            } else {
                tracing::info!(
                    "Unknown event for interface {}: opcode {}",
                    interface,
                    event.header.opcode
                );
            }
        }

        Ok(())
    }

    fn compositor(&self) -> Option<&WlCompositor> {
        self.compositor
            .and_then(|id| self.store.get::<WlCompositor>(&id))
    }
    fn xdg_wm_base(&self) -> Option<&XdgWmBase> {
        self.xdg_wm_base
            .and_then(|id| self.store.get::<XdgWmBase>(&id))
    }

    pub async fn create_window(&mut self) -> Result<(), EventLoopError> {
        self.handle_all_events().await?;

        // At this point, all globals should be registered in the store.
        // We can now create a window using the compositor global.
        let compositor = self.compositor().ok_or(EventLoopError::NoCompositor)?;

        let surface = compositor.create_surface();
        let xdg_surface = self
            .xdg_wm_base()
            .ok_or(EventLoopError::NoCompositor)?
            .xdg_surface(surface.id());

        let toplevel = xdg_surface.toplevel();
        toplevel.set_title("Chugach Window".into());

        surface.commit();

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
            store,
            compositor: None,
            xdg_wm_base: None,
        }))
    }

    pub async fn create_window(&mut self) {
        self.0.create_window().await;
    }

    pub async fn handle_all_events(&mut self) -> Result<(), EventLoopError> {
        self.0.handle_all_events().await
    }
}

#[derive(thiserror::Error, Debug)]
pub enum EventLoopError {
    #[error("Failed to create display connection")]
    ConnectionError(#[from] denali_client::display_connection::DisplayConnectionError),
    #[error("No compositor global found")]
    NoCompositor,
}
