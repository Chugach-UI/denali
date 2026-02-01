//! Core utilities for Denali Wayland.

#![cfg_attr(test, feature(test))]

pub mod connection;
// pub mod encoder;
// pub mod handler;
pub mod id;
pub mod message;
pub mod wire;

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

use message::{DecodeMessageError, Event, IncomingMessage, MessageType, Request};

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

/// Extension methods for interfaces.
pub trait InterfaceExt: Interface {
    /// Attempts to decode an incoming event message for this interface.
    fn try_decode_event(opcode: u16, data: &[u8]) -> Result<Self::Event<'_>, DecodeMessageError> {
        Self::Event::try_decode(Self::INTERFACE, opcode, MessageType::Event, data)
    }

    /// Attempts to decode an incoming request message for this interface.
    fn try_decode_request(
        opcode: u16,
        data: &[u8],
    ) -> Result<Self::Request<'_>, DecodeMessageError> {
        Self::Request::try_decode(Self::INTERFACE, opcode, MessageType::Request, data)
    }
}
impl<I: Interface> InterfaceExt for I {}
