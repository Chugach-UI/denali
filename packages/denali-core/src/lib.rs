//! Core utilities for Denali Wayland.

#![cfg_attr(test, feature(test))]

pub mod handler;
pub mod id_manager;
pub mod wire;
//TODO: Rename and refactor for use in client and server!!!
pub mod proxy;
pub mod store;

// Re-export bitflags for use by denali-macro
// This avoids users of denali-macro from needing to depend on bitflags directly,
// instead they are only required to depend on denali-utils.
#[doc(hidden)]
pub use bitflags as __bitflags;

//TODO: Support client and server
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
