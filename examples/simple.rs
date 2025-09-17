use denali_client::{
    display_connection::DisplayConnection,
    protocol::wayland::wl_registry::{WlRegistry, WlRegistryEvent},
};
use denali_core::store::{InterfaceStore, Store};
use denali_core::
    handler::{Handler, HasStore}
;
use frunk::Coprod;

struct App {
    store: InterfaceStore,
}
impl App {
    pub async fn run(mut self, conn: &mut DisplayConnection) {
        type Ev<'a> = Coprod!(
            WlRegistryEvent<'a>,
        );
        loop {
            if (conn.handle_event::<Ev<'_>, _>(&mut self).await).is_err() {
                break;
            }
        }
    }
}
impl HasStore for App {
    fn store(&self) -> &impl denali_core::store::Store {
        &self.store
    }

    fn store_mut(&mut self) -> &mut impl denali_core::store::Store {
        &mut self.store
    }
}
impl Handler<WlRegistryEvent<'_>> for App {
    fn handle(&mut self, message: WlRegistryEvent, registry: &WlRegistry) {
        match message {
            WlRegistryEvent::Global(ev) => {
                let obj = registry.bind_raw(&ev.interface.data, ev.name, ev.version).unwrap();
                self.store.insert_proxy(ev.interface.data.to_string(), obj.version(), obj);
            }
            WlRegistryEvent::GlobalRemove(ev) => {
                println!("Removed global: {}", ev.name);
            }
        }
    }
}


#[tokio::main]
async fn main() {
    let mut conn = DisplayConnection::new().unwrap();
    let mut store = conn.create_store();
    let disp = conn.display();
    let reg = disp.registry();
    store.insert_interface(reg, 1);

    let app = App {
        store,
    };

    app.run(&mut conn).await;
}
