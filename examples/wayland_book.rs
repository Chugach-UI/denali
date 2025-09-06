use denali_client::{
    display_connection::DisplayConnection,
    protocol::wayland::{
        wl_compositor::WlCompositor,
        wl_registry::{WlRegistry, WlRegistryEvent},
        wl_shm::{WlShm, WlShmEvent},
    },
};
use denali_client_core::Interface;
use denali_core::handler::RawHandler;
use frunk::Coprod;

struct App {
    registry: WlRegistry,
    compositor: Option<WlCompositor>,
    shm: Option<WlShm>,
}

impl App {
    pub async fn run(mut self, connection: DisplayConnection) {
        type Ev<'a> = Coprod!(WlRegistryEvent<'a>, WlShmEvent);
        loop {
            connection.handle_event::<Ev<'_>, _>(&mut self).await;
        }
    }
}

impl RawHandler<WlRegistryEvent<'_>> for App {
    fn handle(
        &mut self,
        message: WlRegistryEvent<'_>,
        object_id: denali_core::wire::serde::ObjectId,
    ) {
        _ = object_id;
        if let WlRegistryEvent::Global(global) = message {
            if global.interface == WlCompositor::INTERFACE {
                self.compositor = Some(self.registry.bind(global.name, 6));
            }
            if global.interface == WlShm::INTERFACE {
                self.shm = Some(self.registry.bind(global.name, 2));
            }
        }
    }
}

impl RawHandler<WlShmEvent> for App {
    fn handle(&mut self, message: WlShmEvent, object_id: denali_core::wire::serde::ObjectId) {
        _ = message;
        _ = object_id;
    }
}

#[tokio::main]
async fn main() {
    let connection = DisplayConnection::new().unwrap();
    let app = App {
        registry: connection.display().registry(),
        compositor: None,
        shm: None,
    };
    app.run(connection).await;
}
