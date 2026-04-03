# Denali

> A 100% Rust implemenation of the Wayland protocol.

<p align="center">
    <img src="https://raw.githubusercontent.com/Chugach-UI/branding/ea55519293ac571bacfea515ccc17ce93e2db82d/Denali/Denali-Banner.svg" alt="Denali banner image" height="200" style="margin-block: 10px;">
</p>

# Features

* [`denali-macro`](packages/denali-macro) provides codegen for arbitrary Wayland protocol (XML) files.
* Alloc-free Wayland Wire serialization and deserialization
* Symetrical server and client support
* Owned object IDs, making use of Rust's borrow checker to prevent API errors
*

# Usage

All messages and events are sent and received from types implementing `Connection`. Currently, only a client-sided connection is provided, but implementing a server-sided connection is possible.

## Opening a connection

To create a client connection, simply call `Connection::new`

```Rust
let (mut conn, display) = Connection::new().await?;
```

## Sending messages

All messages are sent through the `Connection` trait's methods (`send_request`, `send_event`). Each outgoing message has its own struct.

The client and server APIs are perfectly symetrical. This simplifies the Denali codebase and codegen.

```Rust
let registry = conn
        .send_request(WlDisplayGetRegistryRequest { sender: &display })
        .await?
```

```Rust
conn.send_event(WlRegistryGlobalRemoveEvent {
        sender: &registry,
        name: 1,
    })
    .await?
```

# Receiving messages

Because every object ID has a different set of messages and Denali is extendable, message headers must first be peeked and then associated with an object ID.

Incoming messages are received as an enum of possible messages.

```Rust
loop {
    let head = conn.next_header().await.unwrap();

    if head.object_id == xdg_surface {
         let event = conn.decode_message::<XdgSurfaceEvent>().await.unwrap();
    }
}
```
