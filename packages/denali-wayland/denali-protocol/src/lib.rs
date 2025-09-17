pub mod client {
    include!(concat!(env!("OUT_DIR"), "/denali_client_protocols.rs"));
}

pub mod server {
    include!(concat!(env!("OUT_DIR"), "/denali_server_protocols.rs"));
}
