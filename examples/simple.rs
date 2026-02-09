use std::{io::Write, os::fd::OwnedFd};

use denali_client::{
    connection::Connection,
    core::connection::{ClientConnection, Connection as _},
    protocol::{
        wayland::{
            wl_callback::WlCallbackEvent,
            wl_compositor::{WlCompositor, WlCompositorCreateSurfaceRequest},
            wl_display::{WlDisplayGetRegistryRequest, WlDisplaySyncRequest},
            wl_registry::{WlRegistryBindRequest, WlRegistryEvent},
            wl_shm::{Format, WlShm, WlShmCreatePoolRequest},
            wl_shm_pool::WlShmPoolCreateBufferRequest,
            wl_surface::{WlSurfaceAttachRequest, WlSurfaceCommitRequest},
        },
        xdg_shell::{
            xdg_surface::{
                XdgSurfaceAckConfigureRequest, XdgSurfaceEvent, XdgSurfaceGetToplevelRequest,
            },
            xdg_wm_base::{XdgWmBase, XdgWmBaseGetXdgSurfaceRequest},
        },
    },
};
use denali_core::{message::NewIdHint, Interface};
use memfd::MemfdOptions;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BYTES_PER_PIXEL: u32 = 4;

const CAT: &[u8] = include_bytes!("cat.raw");

#[tokio::main]
async fn main() {
    let (mut conn, display) = Connection::new().await.unwrap();

    let registry = conn
        .send_request(WlDisplayGetRegistryRequest { sender: &display })
        .await
        .unwrap();

    let sync_cb = conn
        .send_request(WlDisplaySyncRequest { sender: &display })
        .await
        .unwrap();

    let mut shm = None;
    let mut compositor = None;
    let mut xdg_wm_base = None;

    // Bind our globals
    loop {
        let head = conn.next_header().await.unwrap();

        if head.object_id == sync_cb.get() {
            _ = conn.decode_message::<WlCallbackEvent>().await.unwrap();
            break;
        }

        if head.object_id == registry.get() {
            let event = conn.decode_message::<WlRegistryEvent>().await.unwrap();

            let WlRegistryEvent::Global {
                name, interface, ..
            } = event
            else {
                continue;
            };

            match interface.data.as_ref() {
                WlShm::INTERFACE => {
                    shm = conn
                        .send_request(WlRegistryBindRequest {
                            sender: &registry,
                            name,
                            id: NewIdHint::<WlShm>::new(),
                        })
                        .await
                        .ok();
                }
                WlCompositor::INTERFACE => {
                    compositor = conn
                        .send_request(WlRegistryBindRequest {
                            sender: &registry,
                            name,
                            id: NewIdHint::<WlCompositor>::new(),
                        })
                        .await
                        .ok();
                }
                XdgWmBase::INTERFACE => {
                    xdg_wm_base = conn
                        .send_request(WlRegistryBindRequest {
                            sender: &registry,
                            name,
                            id: NewIdHint::<XdgWmBase>::new(),
                        })
                        .await
                        .ok();
                }
                _ => {}
            }
        }
    }
    println!("All globals bound!");

    let shm = shm.unwrap();
    let compositor = compositor.unwrap();
    let xdg_wm_base = xdg_wm_base.unwrap();

    let surface = conn
        .send_request(WlCompositorCreateSurfaceRequest {
            sender: &compositor,
        })
        .await
        .unwrap();
    let xdg_surface = conn
        .send_request(XdgWmBaseGetXdgSurfaceRequest {
            sender: &xdg_wm_base,
            surface: &surface,
        })
        .await
        .unwrap();
    let xdg_toplevel = conn
        .send_request(XdgSurfaceGetToplevelRequest {
            sender: &xdg_surface,
        })
        .await
        .unwrap();

    conn.send_request(WlSurfaceCommitRequest { sender: &surface })
        .await
        .unwrap();

    let mut configured = false;

    println!("Waiting for surface configuration...");
    // Configure the surface
    while !configured {
        let head = conn.next_header().await.unwrap();

        if head.object_id == xdg_surface.get() {
            let event = conn.decode_message::<XdgSurfaceEvent>().await.unwrap();
            let XdgSurfaceEvent::Configure { serial } = event;

            conn.send_request(XdgSurfaceAckConfigureRequest {
                sender: &xdg_surface,
                serial,
            })
            .await
            .unwrap();

            configured = true;
        }
    }
    println!("Surface configured!");

    let memfd = MemfdOptions::default()
        .create("Denali Wayland Surface Buffer")
        .unwrap();
    let stride = WIDTH * BYTES_PER_PIXEL;
    let size = stride * HEIGHT;

    let mut mem_file = memfd.into_file();
    mem_file.write_all(CAT).unwrap();
    mem_file.set_len(size as _).unwrap();

    let fd: OwnedFd = mem_file.into();

    let pool = conn
        .send_request(WlShmCreatePoolRequest {
            sender: &shm,
            fd,
            size: size as _,
        })
        .await
        .unwrap();
    let buffer = conn
        .send_request(WlShmPoolCreateBufferRequest {
            sender: &pool,
            offset: 0,
            width: WIDTH as _,
            height: HEIGHT as _,
            stride: stride as _,
            format: Format::Argb8888,
        })
        .await
        .unwrap();

    conn.send_request(WlSurfaceAttachRequest {
        sender: &surface,
        buffer: Some(&buffer),
        x: 0,
        y: 0,
    })
    .await
    .unwrap();

    conn.send_request(WlSurfaceCommitRequest { sender: &surface })
        .await
        .unwrap();

    loop {
        let head = conn.next_header().await.unwrap();

        if head.object_id == xdg_surface.get() {
            let event = conn.decode_message::<XdgSurfaceEvent>().await.unwrap();
            let XdgSurfaceEvent::Configure { serial } = event;

            conn.send_request(XdgSurfaceAckConfigureRequest {
                sender: &xdg_surface,
                serial,
            })
            .await
            .unwrap();

            conn.send_request(WlSurfaceCommitRequest { sender: &surface })
                .await
                .unwrap();
        }
    }
}
