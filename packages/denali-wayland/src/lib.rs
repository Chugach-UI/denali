const PROTOCOLS_DIR: &str = concat!(env!("OUT_DIR"), "/protocols");
denali_macro::wayland_protocols!("/usr/share/wayland-protocols");
