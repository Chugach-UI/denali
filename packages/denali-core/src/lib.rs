//! Core utilities for Denali Wayland.

#![cfg_attr(test, feature(test))]

pub mod handler;
pub mod id_manager;
pub mod wire;

// Re-export bitflags for use by denali-macro
// This avoids users of denali-macro from needing to depend on bitflags directly,
// instead they are only required to depend on denali-utils.
#[doc(hidden)]
pub use bitflags as __bitflags;
