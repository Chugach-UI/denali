//! Core utilities for Denali Wayland.

#![cfg_attr(test, feature(test))]
#![feature(unix_socket_ancillary_data)]

pub mod connection;
pub mod fixed;
pub mod id_manager;
pub mod proxy;
pub mod wire;

/// A Wayland object.
pub trait Object: From<proxy::Proxy> {
    /// Get the unique ID of this object.
    fn id(&self) -> u32;
    /// Send a request over the wire associated with this object.
    fn send_request(&self, request: RequestMessage);
}

/// A Wayland interface.
pub trait Interface: Object {
    /// The name of this interface.
    const INTERFACE: &'static str;
    /// The maximum supported version of this interface.
    const MAX_VERSION: u32;
}

// Re-export bitflags for use by denali-macro
// This avoids users of denali-macro from needing to depend on bitflags directly,
// instead they are only required to depend on denali-utils.
#[doc(hidden)]
pub use bitflags as __bitflags;
use proxy::RequestMessage;
