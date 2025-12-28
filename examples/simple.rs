use denali_client::connection::Connection;
use denali_client::core::connection::Connection as _;
use denali_client::protocol::wayland::wl_display::WlDisplay;
use denali_core::handler::EventHandler;
use denali_core::message::{Event, OutgoingMessage};

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

    struct EventType;
    impl OutgoingMessage<Event> for EventType {
        type Interface = WlDisplay;
        type Response = ();

        fn encode(&self, data: &mut [u8]) -> Result<(), denali_core::message::EncodeMessageError> {
            todo!()
        }
        fn encoded_size(&self) -> usize {
            todo!()
        }
    }

    conn.send_event(&display, EventType);
}
