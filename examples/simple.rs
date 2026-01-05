use std::cell::RefCell;
use std::rc::Rc;

use denali_client::connection::Connection;
use denali_client::core::connection::Connection as _;
use denali_client::protocol::wayland::wl_display::{
    WlDisplay, WlDisplayEvent, WlDisplayGetRegistryRequest,
};
use denali_client::protocol::wayland::wl_output::WlOutput;
use denali_client::protocol::wayland::wl_registry::{
    WlRegistry, WlRegistryBindRequest, WlRegistryEvent,
};
use denali_client::protocol::wlr_gamma_control_unstable_v1::zwlr_gamma_control_manager_v1::{
    ZwlrGammaControlManagerV1, ZwlrGammaControlManagerV1GetGammaControlRequest,
};
use denali_client::protocol::wlr_gamma_control_unstable_v1::zwlr_gamma_control_v1::{
    ZwlrGammaControlV1, ZwlrGammaControlV1Event, ZwlrGammaControlV1SetGammaRequest,
};
use denali_core::connection::ClientConnection;
use denali_core::handler::event_handler;
use denali_core::id::ObjectId;
use denali_core::message::NewIdHint;
use denali_core::Interface;

#[tokio::main]
async fn main() {
    let (mut conn, display) = Connection::new().unwrap();

    conn.add_handler(
        &display,
        event_handler::<WlDisplay, _>(|ev, _| async {
            match ev {
                WlDisplayEvent::Error {
                    object_id,
                    code,
                    message,
                } => {
                    eprintln!(
                        "Display error on object {}: code {}, message: {}",
                        object_id.get(),
                        code,
                        message.data
                    );
                }
                WlDisplayEvent::DeleteId { id } => {
                    println!("Delete ID: {}", id);
                }
            }
        }),
    );

    let reg = conn
        .send_request(WlDisplayGetRegistryRequest { sender: &display })
        .await
        .unwrap();

    conn.add_handler(
        &reg,
        event_handler::<WlRegistry, _>(move |ev, reg| async move {
            match ev {
                WlRegistryEvent::Global {
                    name,
                    interface,
                    version,
                } => {
                    println!(
                        "Global object announced: name={}, interface={}, version={}",
                        name, interface.data, version
                    );
                }
                WlRegistryEvent::GlobalRemove { name } => {
                    println!("Global object removed: name={}", name);
                }
            }
        }),
    );

    conn.handle_events().await.unwrap();
}
