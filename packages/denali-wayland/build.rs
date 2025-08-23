use std::env;
use std::fs;
use std::path::Path;

use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use tar::Archive;

const WL_LOCKS: &str = include_str!("./wayland.lock");

pub fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let lock_lines: Vec<&str> = WL_LOCKS.lines().collect();
    let wayland_commit = lock_lines[0].replace("wayland=", "");
    let wayland_protocols_commit = lock_lines[1].replace("wayland-protocols=", "");

    let wayland_xml_path = format!(
        "https://gitlab.freedesktop.org/wayland/wayland/-/raw/{}/protocol/wayland.xml",
        wayland_commit
    );
    let wayland_protocols_archive_paths: Vec<String> = ["stable", "staging", "unstable", "experimental"]
        .iter()
        .map(|path| {
            format!(
                "https://gitlab.freedesktop.org/wayland/wayland-protocols/-/archive/{commit}/wayland-protocols-{commit}.tar.gz?path={path}",
                commit = wayland_protocols_commit
            )
        })
        .collect();

    let protocol_blacklist = [
        "linux-dmabuf-unstable-v1.xml",
        "tablet-unstable-v1.xml",
        "tablet-unstable-v2.xml",
        "text-input-unstable-v1.xml",
        "xdg-foreign-unstable-v1.xml",
        "xdg-shell-unstable-v5.xml",
        "xdg-shell-unstable-v6.xml",
    ];

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let protocols_path = Path::new(&out_dir).join("protocols");
    fs::create_dir_all(&protocols_path).unwrap();

    let client = Client::new();

    let wayland_xml_bytes = client
        .get(wayland_xml_path)
        .send()
        .unwrap()
        .bytes()
        .unwrap();

    fs::write(protocols_path.join("wayland.xml"), wayland_xml_bytes).unwrap();

    for archive_path in wayland_protocols_archive_paths {
        let archive_bytes = client.get(archive_path).send().unwrap().bytes().unwrap();

        let tar = GzDecoder::new(&archive_bytes[..]);
        let mut archive = Archive::new(tar);

        for entry in archive.entries().unwrap() {
            let mut unwrapped = entry.unwrap();
            let path = unwrapped.path().unwrap();
            if let Some(ext) = path.extension()
                && let Some(name) = path.file_name()
                && ext == "xml"
                && !protocol_blacklist.contains(&name.to_string_lossy().into_owned().as_str())
            {
                unwrapped.unpack(protocols_path.join(name)).unwrap();
            }
        }
    }

    let generated_code_path = Path::new(&out_dir).join("wayland_client_protocols.rs");

    fs::write(
        generated_code_path,
        format!(
            "denali_macro::wayland_protocols!(\"{}\")",
            protocols_path.to_string_lossy()
        ),
    )
    .unwrap();
}
