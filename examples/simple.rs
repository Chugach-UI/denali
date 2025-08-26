use std::time::Duration;

use denali_wayland::{
    display_connection::DisplayConnection,
    protocol::wayland::{wl_compositor::WlCompositor, wl_shm::WlShm},
};

#[tokio::main]
async fn main() {
    let conn = DisplayConnection::new().await.unwrap();
    let disp = conn.display();
    let reg = disp.registry();
    _ = reg;

    loop {
        let ev = conn.get_event().await;
        println!("{ev:?}");
    }
}
