//! Denali core utilities only pertaining to the client side of the wayland protocol.

#![feature(unix_socket_ancillary_data)]
#![feature(atomic_from_mut)]

pub mod connection;
mod stream;
