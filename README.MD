# Denali

> A 100% Rust implementation of the Wayland protocol.

<p align="center">
    <img src="https://raw.githubusercontent.com/Chugach-UI/branding/ea55519293ac571bacfea515ccc17ce93e2db82d/Denali/Denali-Banner.svg" alt="Denali banner image" height="200" style="margin-block: 10px;">
</p>

# Features

* [`denali-macro`](packages/denali-macro) provides codegen for arbitrary Wayland protocol (XML) files.
* Alloc-free Wayland wire serialization and deserialization
* Symmetrical server and client support
* Owned object IDs, making use of Rust's borrow checker to prevent API errors

# Usage

All messages and events are sent and received from types implementing `Connection`. Currently, only a client-sided connection is provided, but implementing a server-sided connection is possible.

## Opening a connection

To create a client connection, call `Connection::new`:

```rust
let (mut conn, display) = Connection::new().await?;
```

## Binding globals

The `Registry` helper performs the initial roundtrip and collects all advertised globals. Typed `bind` / `try_bind` methods handle version negotiation automatically:

```rust
let registry = Registry::new(&mut conn, &display).await?;

let shm        = registry.bind::<WlShm>(&mut conn).await?;
let compositor = registry.bind::<WlCompositor>(&mut conn).await?;
// Optional globals return None instead of an error:
let dmabuf     = registry.try_bind::<ZwpLinuxDmabufV1>(&mut conn).await?;
```

## Sending messages

All messages are sent through the `Connection` trait's methods (`send_request`, `send_event`). Each outgoing message has its own struct.

The client and server APIs are perfectly symmetrical. This simplifies the Denali codebase and codegen.

```rust
let surface = conn
    .send_request(WlCompositorCreateSurfaceRequest {
        sender: &compositor,
    })
    .await?;
```

```rust
conn.send_event(WlRegistryGlobalRemoveEvent {
        sender: &registry,
        name: 1,
    })
    .await?;
```

## Receiving messages

### Single object

`recv_event` waits for the next event on a specific object. The event type is inferred from the object's interface:

```rust
let XdgSurfaceEvent::Configure { serial } =
    conn.recv_event(&xdg_surface).await?;
```

The server-side equivalent is `recv_request`.

### Multiple objects

Use `try_recv_event` with a peeked header to dispatch events from several objects:

```rust
loop {
    let head = conn.next_header().await?;

    if let Some(event) = conn.try_recv_event(&head, &xdg_surface).await? {
        // handle xdg_surface events
    } else if let Some(event) = conn.try_recv_event(&head, &output).await? {
        // handle wl_output events
    }
}
```

### Low-level

You can also peek headers and decode manually:

```rust
let head = conn.next_header().await?;
let event = conn.decode_message::<XdgSurfaceEvent>().await?;
```
