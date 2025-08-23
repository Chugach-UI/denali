#![cfg_attr(test, feature(test))]

pub mod fixed;
pub mod proxy;
pub mod wire;

pub trait Object: From<proxy::Proxy> {}
pub trait Interface: Object {
    const INTERFACE: &'static str;
    const MAX_VERSION: u32;
}

// Re-export bitflags for use by denali-macro
// This avoids users of denali-macro from needing to depend on bitflags directly,
// instead they are only required to depend on denali-utils.
#[doc(hidden)]
pub use bitflags as __bitflags;

