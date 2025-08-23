pub mod display_connection;

denali_macro::wayland_protocols!(
    "/home/gavin/Dev/rust/denali/target/debug/build/denali-wayland-2d17737998a5d450/out/protocols"
);

fn test() {
    use denali_utils::wire::serde::Encode;

    let capabilities_message =
        ext_background_effect_v1::ext_background_effect_manager_v1::CapabilitiesEvent {
            flags: ext_background_effect_v1::ext_background_effect_manager_v1::Capability::BLUR,
        };

    let mut buffer = [0u8; 64];
    capabilities_message.encode(&mut buffer).unwrap();

    println!("Encoded message: {:?}", buffer);
}