use denali_client::{
    connection::Connection,
    core::connection::{ClientConnection, Connection as _},
    protocol::wayland::{
        wl_display::WlDisplayGetRegistryRequest,
        wl_registry::{WlRegistry, WlRegistryEvent},
    },
};
use denali_core::InterfaceExt;

#[tokio::main]
async fn main() {
    let (mut conn, display) = Connection::new().unwrap();

    let registry = conn
        .send_request(WlDisplayGetRegistryRequest { sender: &display })
        .await
        .unwrap();

    loop {
        let message = conn.next_message().await.unwrap();

        match message.object_id {
            _ if message.object_id == registry.get() => {
                let event = WlRegistry::try_decode_event(message.opcode, &message.body).unwrap();
                handle_registry_event(event, &mut conn).await;
            }
            _ => {}
        }
    }
}

async fn handle_registry_event(event: WlRegistryEvent<'_>, conn: &mut Connection) {
    match event {
        WlRegistryEvent::Global {
            name,
            interface,
            version,
        } => {
            println!(
                "New global announced! {name}: {}_v{version}",
                interface.data
            )
        }
        WlRegistryEvent::GlobalRemove { name } => {
            println!("Global deleted: {name}")
        }
    }
}
