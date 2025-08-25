#![cfg_attr(test, feature(test))]

pub mod fixed;
pub mod id_manager;
pub mod proxy;
pub mod socket;
pub mod wire;

pub trait Object: From<proxy::Proxy> {
    fn id(&self) -> u32;
    fn send_request(&self, request: RequestMessage);
}
pub trait Interface: Object {
    const INTERFACE: &'static str;
    const MAX_VERSION: u32;
}

// Re-export bitflags for use by denali-macro
// This avoids users of denali-macro from needing to depend on bitflags directly,
// instead they are only required to depend on denali-utils.
#[doc(hidden)]
pub use bitflags as __bitflags;
use proxy::RequestMessage;
