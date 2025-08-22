#![cfg_attr(test, feature(test))]

pub mod fixed;
pub mod wire;

// Re-export bitflags for use by denali-macro
// This avoids users of vexide-macro from needing to depend on bitflags directly,
// instead they are only required to depend on denali-utils.
#[doc(hidden)]
pub use bitflags as __bitflags;