//! An async proxy for a wayland object.

use denali_core::wire::ObjectId;

use crate::connection::AsyncConnection;

pub struct AsyncProxy {
    id: ObjectId,
    connection: AsyncConnection,
}
