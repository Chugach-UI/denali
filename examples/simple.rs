#[tokio::main]
async fn main() {
    use denali_wayland::display_connection::DisplayConnection;
    let conn = DisplayConnection::new().await.unwrap();
    let disp = conn.display();
    _ = disp;
}
