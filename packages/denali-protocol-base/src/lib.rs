//! Codegen for common Wayland protocols.
//!
//! This includes protocols from:
//! - WLR,
//! - unstable,
//! - core

#![allow(missing_docs)]

include!(concat!(env!("OUT_DIR"), "/protocols.rs"));
