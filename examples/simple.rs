use denali_client::{
    display_connection::DisplayConnection,
    protocol::{
        wayland::{
            wl_compositor::WlCompositor,
            wl_display::WlDisplayEvent,
            wl_registry::{WlRegistry, WlRegistryEvent},
        },
        wlr_foreign_toplevel_management_unstable_v1::{
            zwlr_foreign_toplevel_handle_v1::{
                ZwlrForeignToplevelHandleV1, ZwlrForeignToplevelHandleV1Event,
            },
            zwlr_foreign_toplevel_manager_v1::{
                ZwlrForeignToplevelManagerV1, ZwlrForeignToplevelManagerV1Event,
            },
        },
    },
};
use denali_client_core::{store::Store, Interface};
use denali_core::handler::Handler;
use frunk::Coprod;

struct App {
    registry: WlRegistry,
    store: Store,
}
impl App {
    pub async fn run(mut self, conn: DisplayConnection) {
        type Ev<'a> = Coprod!(
            WlRegistryEvent<'a>,
            WlDisplayEvent<'a>,
            ZwlrForeignToplevelManagerV1Event,
            ZwlrForeignToplevelHandleV1Event<'a>
        );
        loop {
            if let Err(_) = conn.handle_event::<Ev<'_>, _>(&mut self).await {
                break;
            }
        }
    }
}
impl Handler<WlRegistryEvent<'_>> for App {
    fn handle(&mut self, message: WlRegistryEvent, _object_id: denali_core::wire::serde::ObjectId) {
        match message {
            WlRegistryEvent::Global(ev) => {
                if ev.interface.data == ZwlrForeignToplevelManagerV1::INTERFACE {
                    let mgr = self
                        .registry
                        .bind::<ZwlrForeignToplevelManagerV1>(ev.name, ev.version);
                    self.store.insert_interface(mgr, ev.version);
                }
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

impl Handler<ZwlrForeignToplevelManagerV1Event> for App {
    fn handle(
        &mut self,
        message: ZwlrForeignToplevelManagerV1Event,
        object_id: denali_core::wire::serde::ObjectId,
    ) {
        match message {
            ZwlrForeignToplevelManagerV1Event::Toplevel(toplevel_event) => {
                self.store.insert(
                    toplevel_event.toplevel,
                    1,
                    ZwlrForeignToplevelHandleV1::INTERFACE.to_string(),
                );
            }
            ZwlrForeignToplevelManagerV1Event::Finished(finished_event) => {
                println!("Foreign toplevel manager finished: {:?}", finished_event);
            }
        }
    }
}
impl Handler<ZwlrForeignToplevelHandleV1Event<'_>> for App {
    fn handle(
        &mut self,
        message: ZwlrForeignToplevelHandleV1Event,
        object_id: denali_core::wire::serde::ObjectId,
    ) {
        let Some(handle) = self.store.get::<ZwlrForeignToplevelHandleV1>(&object_id) else {
            return;
        };

        handle.close();

        match message {
            ZwlrForeignToplevelHandleV1Event::Title(title_event) => {
                println!("Toplevel title changed: {}", title_event.title.data);
            }
            ZwlrForeignToplevelHandleV1Event::AppId(app_id_event) => {
                println!("Toplevel app_id changed: {}", app_id_event.app_id.data);
            }
            ZwlrForeignToplevelHandleV1Event::OutputEnter(output_enter_event) => {}
            ZwlrForeignToplevelHandleV1Event::OutputLeave(output_leave_event) => {}
            ZwlrForeignToplevelHandleV1Event::State(state_event) => {}
            ZwlrForeignToplevelHandleV1Event::Done(done_event) => {}
            ZwlrForeignToplevelHandleV1Event::Closed(closed_event) => {}
            ZwlrForeignToplevelHandleV1Event::Parent(parent_event) => {}
        }
    }
}

#[tokio::main]
async fn main() {
    let mut conn = DisplayConnection::new().unwrap();
    let store = conn.create_store();
    let disp = conn.display();
    let reg = disp.registry();

    let app = App {
        registry: reg,
        store,
    };

    app.run(&mut conn).await;
}
