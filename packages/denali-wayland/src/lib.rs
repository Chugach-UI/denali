pub mod display_connection;
pub mod protocol;

async fn _test() {
    use display_connection::DisplayConnection;
    let conn = DisplayConnection::new().await.unwrap();
    let disp = conn.display();
    _ = disp;
}
