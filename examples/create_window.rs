use std::time::Duration;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    tracing::info!("Creating ev loop...");
    let mut event_loop = chugach_windowing::event_loop::EventLoop::new().unwrap();
    tracing::info!("Creating window...");
    event_loop.create_window().await;
    tracing::info!("Created window...");

    loop {
        event_loop.handle_all_events().await.unwrap();
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}