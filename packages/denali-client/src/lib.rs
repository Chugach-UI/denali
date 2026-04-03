//! Utilities for connecting and interacting with Wayland servers.
//!
//! See [`Connection`](connection::Connection)

pub use denali_core as core;
pub use denali_protocol_base as protocol;

pub mod connection;
pub mod registry;
