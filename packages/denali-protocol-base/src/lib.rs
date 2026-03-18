//! Codegen for common Wayland protocols.
//!
//! This includes protocols from:
//! - WLR,
//! - unstable,
//! - core

include!(concat!(env!("OUT_DIR"), "/protocols.rs"));
