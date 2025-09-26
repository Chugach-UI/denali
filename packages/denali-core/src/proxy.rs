//! Proxy for a Wayland object.

use crate::{connection::Connection, wire::ObjectId};

/// Client-side proxy for a wayland object.
pub struct Proxy {
    /// The object id of the proxy.
    id: ObjectId,
    connection: Connection,
}
