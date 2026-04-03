use std::{
    io::{Seek, Write},
    os::fd::OwnedFd,
};

use denali_client::{
    connection::Connection,
    core::connection::{ClientConnection, Connection as _},
    protocol::{
        wayland::{
            wl_callback::WlCallbackEvent,
            wl_display::{WlDisplayGetRegistryRequest, WlDisplaySyncRequest},
            wl_output::WlOutput,
            wl_registry::{WlRegistryBindRequest, WlRegistryEvent},
        },
        wlr_gamma_control_unstable_v1::{
            zwlr_gamma_control_manager_v1::{
                ZwlrGammaControlManagerV1,
                ZwlrGammaControlManagerV1GetGammaControlRequest,
            },
            zwlr_gamma_control_v1::{ZwlrGammaControlV1Event, ZwlrGammaControlV1SetGammaRequest},
        },
    },
};
use denali_core::{message::NewIdHint, Interface};
use memfd::MemfdOptions;

/// Gamma increase factor. 1.0 = no change, >1.0 = brighter midtones (higher gamma).
const GAMMA: f64 = 5.0;

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

    let mut gamma_manager = None;
    let mut output = None;

    // Bind globals
    loop {
        let head = conn.next_header().await.unwrap();

        if head.object_id == sync_cb {
            _ = conn.decode_message::<WlCallbackEvent>().await.unwrap();
            break;
        }

        if head.object_id == registry {
            let event = conn.decode_message::<WlRegistryEvent>().await.unwrap();

            let WlRegistryEvent::Global {
                name,
                interface,
                version,
            } = event
            else {
                continue;
            };

            match interface.as_ref() {
                ZwlrGammaControlManagerV1::INTERFACE => {
                    gamma_manager = conn
                        .send_request(WlRegistryBindRequest {
                            sender: &registry,
                            name,
                            id: NewIdHint::<ZwlrGammaControlManagerV1>::new(version),
                        })
                        .await
                        .ok();
                }
                WlOutput::INTERFACE => {
                    // Bind to the first output we find
                    if output.is_none() {
                        output = conn
                            .send_request(WlRegistryBindRequest {
                                sender: &registry,
                                name,
                                id: NewIdHint::<WlOutput>::new(version),
                            })
                            .await
                            .ok();
                    }
                }
                _ => {}
            }
        }
    }

    let gamma_manager = gamma_manager.expect("compositor does not support wlr-gamma-control");
    let output = output.expect("no outputs found");

    // Do a second roundtrip to drain wl_output info events (Geometry, Mode, Done, etc.)
    // before using the output for gamma control.
    let sync_cb = conn
        .send_request(WlDisplaySyncRequest { sender: &display })
        .await
        .unwrap();

    loop {
        let head = conn.next_header().await.unwrap();

        if head.object_id == sync_cb.get() {
            _ = conn.decode_message::<WlCallbackEvent>().await.unwrap();
            break;
        }
    }

    println!("Bound gamma control manager and output");

    // Create a gamma control for the output
    let gamma_control = conn
        .send_request(ZwlrGammaControlManagerV1GetGammaControlRequest {
            sender: &gamma_manager,
            output: &output,
        })
        .await
        .unwrap();

    // Wait for the gamma_size event
    let gamma_size = loop {
        let head = conn.next_header().await.unwrap();

        if head.object_id == gamma_control.get() {
            let event = conn
                .decode_message::<ZwlrGammaControlV1Event>()
                .await
                .unwrap();

            match event {
                ZwlrGammaControlV1Event::GammaSize { size } => break size,
                ZwlrGammaControlV1Event::Failed {} => {
                    panic!("gamma control failed (output may not support it or another client has exclusive access)");
                }
            }
        }
    };

    println!("Gamma ramp size: {gamma_size}");

    // Build the gamma table: three ramps (R, G, B), each `gamma_size` u16 values.
    // Applying an inverse gamma curve (pow(1/GAMMA)) makes midtones brighter,
    // which is the conventional meaning of "increasing gamma".
    let table_bytes = gamma_size as usize * 3 * size_of::<u16>();

    let mut table = Vec::with_capacity(gamma_size as usize * 3);
    for _ in 0..3 {
        for i in 0..gamma_size {
            let linear = i as f64 / (gamma_size - 1) as f64;
            let corrected = linear.powf(1.0 / GAMMA);
            let value = (corrected * u16::MAX as f64).round() as u16;
            table.push(value);
        }
    }

    // Write the table as raw u16 bytes into a memfd
    let bytes: Vec<u8> = table.iter().flat_map(|v| v.to_ne_bytes()).collect();
    assert_eq!(bytes.len(), table_bytes);

    let memfd = MemfdOptions::default()
        .create("gamma-table")
        .unwrap();
    let mut mem_file = memfd.into_file();
    mem_file.write_all(&bytes).unwrap();
    mem_file.rewind().unwrap();

    let fd: OwnedFd = mem_file.into();

    conn.send_request(ZwlrGammaControlV1SetGammaRequest {
        sender: &gamma_control,
        fd,
    })
    .await
    .unwrap();

    loop {
        let head = conn.next_header().await.unwrap();

        if head.object_id == gamma_control.get() {
            let event = conn
                .decode_message::<ZwlrGammaControlV1Event>()
                .await
                .unwrap();

            if let ZwlrGammaControlV1Event::Failed {} = event {
                eprintln!("Gamma control lost");
                break;
            }
        }
    }
}
