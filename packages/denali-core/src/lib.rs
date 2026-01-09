//! Core utilities for Denali Wayland.

#![cfg_attr(test, feature(test))]

pub mod connection;
pub mod handler;
pub mod id;
pub mod message;
pub mod wire;
pub mod encoder;

mod sealed {
    pub trait Sealed {}
}

pub mod prelude {
    pub use crate::Interface;
    pub use crate::connection::Connection;
    pub use crate::id::{AnyObjectId, ObjectId};
    pub use crate::message::{IncomingMessage, OutgoingMessage};
}

//TODO: Rename and refactor for use in client and server!!!
// pub mod proxy;
// pub mod store;

// Re-export bitflags for use by denali-macro
// This avoids users of denali-macro from needing to depend on bitflags directly,
// instead they are only required to depend on denali-utils.
#[doc(hidden)]
pub use bitflags as __bitflags;

use crate::message::{Event, IncomingMessage, Request};

/// A Wayland interface.
pub trait Interface {
    /// The name of this interface.
    const INTERFACE: &'static str;
    /// The maximum supported version of this interface.
    const MAX_VERSION: u32;

    /// The event type for this interface.
    type Event<'a>: IncomingMessage<Event, Interface = Self>;
    /// The request type for this interface.
    type Request<'a>: IncomingMessage<Request, Interface = Self>;
}
