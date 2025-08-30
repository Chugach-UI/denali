use std::time::Duration;

use denali_core::{handler::{Handler, Message}, Interface};
use denali_wayland::{
    display_connection::DisplayConnection,
    protocol::wayland::{
        wl_compositor::WlCompositor,
        wl_display::{WlDisplay, WlDisplayEvent},
        wl_registry::{WlRegistry, WlRegistryEvent},
        wl_shm::WlShm,
    },
};
use frunk::Coprod;

struct App {
    registry: WlRegistry,
}
impl App {
    pub async fn run(mut self, conn: DisplayConnection) {
        type Ev<'a> = Coprod!(WlRegistryEvent<'a>, WlDisplayEvent<'a>);
        loop {
            conn.handle_event::<Ev<'_>, _>(&mut self).await;
        }
    }
}
impl Handler<WlRegistryEvent<'_>> for App {
    fn handle(&mut self, message: WlRegistryEvent, _object_id: denali_core::wire::serde::ObjectId) {
        match message {
            WlRegistryEvent::Global(ev) => {
                println!("New global: {} v{}", ev.interface.data, ev.version);
            }
            WlRegistryEvent::GlobalRemove(ev) => {
                println!("Removed global: {}", ev.name);
            }
        }
    }
}
impl Handler<WlDisplayEvent<'_>> for App {
    fn handle(&mut self, message: WlDisplayEvent, object_id: denali_core::wire::serde::ObjectId) {
        match message {
            WlDisplayEvent::Error(error_event) => {
                eprintln!(
                    "Display error on object {}: code {}, message: {}",
                    object_id, error_event.code, error_event.message.data
                );
            }
            WlDisplayEvent::DeleteId(delete_id_event) => {
                println!("Display deleted id: {}", delete_id_event.id);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let conn = DisplayConnection::new().await.unwrap();
    let disp = conn.display();
    let reg = disp.registry();

    let app = App {
        registry: reg,
    };

    app.run(conn).await;
}
