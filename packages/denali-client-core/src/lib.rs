//! Denali core utilities only pertaining to the client side of the wayland protocol.

#![feature(unix_socket_ancillary_data)]
#![feature(atomic_from_mut)]

pub mod connection;
pub mod proxy;
pub mod store;
mod stream;

/// A Wayland object.
pub trait Object: From<proxy::Proxy> + Into<proxy::Proxy> {
    /// Get the unique ID of this object.
    fn id(&self) -> u32;
    /// Send a request over the wire associated with this object.
    fn send_request(&self, request: proxy::RequestMessage);
}

/// A Wayland interface.
pub trait Interface: Object {
    /// The name of this interface.
    const INTERFACE: &'static str;
    /// The maximum supported version of this interface.
    const MAX_VERSION: u32;
}
