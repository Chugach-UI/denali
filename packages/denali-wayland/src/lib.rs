denali_macro::wayland_protocols!(
    "/home/gavin/Dev/rust/denali/target/debug/build/denali-wayland-b3c61ac19c7e8fb4/out/protocols/ext-background-effect-v1.xml"
);

fn test() {
    let capabilities_message = ext_background_effect_v_1::Capabilities {
        flags: 0,
    };

    let mut buffer = [0u8; 64];
    capabilities_message.encode(&mut buffer).unwrap();

    println!("Encoded message: {:?}", buffer);
}