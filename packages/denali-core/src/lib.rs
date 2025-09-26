//! Core utilities for Denali Wayland.

#![cfg_attr(test, feature(test))]
#![feature(const_trait_impl)]

pub mod connection;
pub mod fixed;
pub mod id_manager;
pub mod proxy;
pub mod wire;

/// A Wayland interface.
pub trait Interface {
    /// The name of this interface.
    const INTERFACE: &'static str;
    /// The maximum supported version of this interface.
    const MAX_VERSION: u32;
}
