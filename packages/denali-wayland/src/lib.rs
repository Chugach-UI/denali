pub mod display_connection;
pub mod protocol;

fn _test() {
    use display_connection::DisplayConnection;
    let conn = DisplayConnection::new().unwrap();
    let disp = conn.display();
    _ = disp;
}

// denali_macro::wayland_protocols!(
//     "/home/gavin/Dev/rust/denali/target/debug/build/denali-wayland-e9b94f057ff0180e/out/protocols/wayland.xml"
// );
