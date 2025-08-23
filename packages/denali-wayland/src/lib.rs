pub mod display_connection;
pub mod protocol;

fn _test() {
    use display_connection::DisplayConnection;
    let conn = DisplayConnection::new().unwrap();
    let disp = conn.display();
    _ = disp;
}
