use denali_client::connection::Connection;
use denali_client::core::connection::Connection as _;
use denali_client::protocol::wayland::wl_compositor::WlCompositor;
use denali_client::protocol::wayland::wl_display::{
    WlDisplay, WlDisplayEvent, WlDisplayGetRegistryRequest, WlDisplayRequest,
};
use denali_client::protocol::wayland::wl_registry::{
    WlRegistry, WlRegistryBindRequest, WlRegistryEvent,
};
use denali_core::handler::EventHandler;
use denali_core::message::NewIdHint;

#[tokio::main]
async fn main() {
    let (mut conn, display) = Connection::new().unwrap();

    conn.add_handler(
        &display,
        EventHandler::<WlDisplay, _>::new(move |ev| match ev {
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
        }),
    );

    let reg = conn
        .send_request(WlDisplayGetRegistryRequest { sender: &display })
        .await
        .unwrap();

    conn.add_handler(
        &reg,
        EventHandler::<WlRegistry, _>::new(|ev| match ev {
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
        }),
    );

    conn.handle_events().await.unwrap();
}
