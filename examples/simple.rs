use denali_client::connection::Connection;
use denali_client::core::connection::Connection as _;
use denali_client::protocol::wayland::wl_display::{
    WlDisplay, WlDisplayGetRegistryRequest, WlDisplayRequest,
};
use denali_core::handler::EventHandler;

#[tokio::main]
async fn main() {
    let (mut conn, display) = Connection::new().unwrap();

    let mut ev_count = 0;
    conn.add_handler(
        &display,
        EventHandler::<WlDisplay, _>::new(move |_ev| {
            ev_count += 1;
            println!("Event count: {}", ev_count)
        }),
    );

    conn.send_request(WlDisplayGetRegistryRequest {})
}
