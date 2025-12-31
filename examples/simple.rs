use std::cell::RefCell;
use std::rc::Rc;

use denali_client::connection::{event_handler, Connection};
use denali_client::core::connection::Connection as _;
use denali_client::protocol::wayland::wl_display::{
    WlDisplay, WlDisplayEvent, WlDisplayGetRegistryRequest,
};
use denali_client::protocol::wayland::wl_output::WlOutput;
use denali_client::protocol::wayland::wl_registry::{
    WlRegistry, WlRegistryBindRequest, WlRegistryEvent,
};
use denali_client::protocol::wlr_gamma_control_unstable_v1::zwlr_gamma_control_manager_v1::{
    ZwlrGammaControlManagerV1, ZwlrGammaControlManagerV1GetGammaControlRequest,
};
use denali_client::protocol::wlr_gamma_control_unstable_v1::zwlr_gamma_control_v1::{
    ZwlrGammaControlV1, ZwlrGammaControlV1Event, ZwlrGammaControlV1SetGammaRequest,
};
use denali_core::id::ObjectId;
use denali_core::message::NewIdHint;
use denali_core::Interface;

#[tokio::main]
async fn main() {
    let (mut conn, display) = Connection::new().unwrap();

    conn.add_handler(
        &display,
        event_handler::<WlDisplay, _>(|ev, _, _| {
            Box::pin(async {
                match ev {
                    WlDisplayEvent::Error {
                        object_id,
                        code,
                        message,
                    } => {
                        eprintln!(
                            "Display error on object {}: code {}, message: {}",
                            object_id.get(),
                            code,
                            message.data
                        );
                    }
                    WlDisplayEvent::DeleteId { id } => {
                        println!("Delete ID: {}", id);
                    }
                }
            })
        }),
    );

    let reg = conn
        .send_request(WlDisplayGetRegistryRequest { sender: &display })
        .await
        .unwrap();

    let gamma_mgr = Rc::new(RefCell::new(None));
    conn.add_handler(
        &reg,
        event_handler::<WlRegistry, _>(move |ev, conn, reg| {
            let gamma_mgr = gamma_mgr.clone();
            Box::pin(async move {
                match ev {
                    WlRegistryEvent::Global {
                        name,
                        interface,
                        version,
                    } => {
                        println!(
                            "Global object announced: name={}, interface={}, version={}",
                            name, interface.data, version
                        );

                        if interface.data == ZwlrGammaControlManagerV1::INTERFACE {
                            let new_gamma_mgr = conn
                                .send_request(WlRegistryBindRequest {
                                    sender: &reg,
                                    name,
                                    id: NewIdHint::<ZwlrGammaControlManagerV1>::new(),
                                })
                                .await
                                .unwrap();
                            gamma_mgr.borrow_mut().replace(new_gamma_mgr);
                        } else if interface.data == WlOutput::INTERFACE {
                            let output = conn
                                .send_request(WlRegistryBindRequest {
                                    sender: &reg,
                                    name,
                                    id: NewIdHint::<WlOutput>::new(),
                                })
                                .await
                                .unwrap();

                            if let Some(gamma_mgr) = gamma_mgr.borrow().as_ref() {
                                handle_output(conn, &gamma_mgr, &output).await;
                            }
                        }
                    }
                    WlRegistryEvent::GlobalRemove { name } => {
                        println!("Global object removed: name={}", name);
                    }
                }
            })
        }),
    );

    conn.handle_events().await.unwrap();
}

async fn handle_output(
    conn: &mut Connection<'_>,
    gamma_mgr: &ObjectId<ZwlrGammaControlManagerV1>,
    output: &ObjectId<WlOutput>,
) {
    let control = conn
        .send_request(ZwlrGammaControlManagerV1GetGammaControlRequest {
            sender: &gamma_mgr,
            output,
        })
        .await
        .unwrap();

    conn.add_handler(
        &control,
        event_handler::<ZwlrGammaControlV1, _>(move |ev, _conn, _reg| {
            Box::pin(async move {
                match ev {
                    ZwlrGammaControlV1Event::GammaSize { size } => {
                        println!("gamma size for output: {size}")
                    }
                    ZwlrGammaControlV1Event::Failed {} => todo!(),
                }
            })
        }),
    );
}
