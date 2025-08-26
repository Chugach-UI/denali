use denali_wayland::{
    display_connection::DisplayConnection, protocol::wayland::wl_compositor::WlCompositor,
};

#[tokio::main]
async fn main() {
    let conn = DisplayConnection::new().await.unwrap();
    let disp = conn.display();
    let reg = disp.registry();
    let comp: WlCompositor = reg.bind(1, 6);
    let surf = comp.create_surface();
    surf.destroy();
}