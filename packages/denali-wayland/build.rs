use std::env;
use std::fs;
use std::path::Path;

use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use tar::Archive;

pub fn main() {
    let waylalnd_xml_path = "https://gitlab.freedesktop.org/wayland/wayland/-/blob/9b169ff945a8fdddc3a92b1990bddc29a7d24465/protocol/wayland.xml";
    let wayland_protocols_archive_paths = [
        "https://gitlab.freedesktop.org/wayland/wayland-protocols/-/archive/0091197f5c1b1f2c131f1410e99f9c95d50646be/wayland-protocols-0091197f5c1b1f2c131f1410e99f9c95d50646be.tar.gz?path=stable",
        "https://gitlab.freedesktop.org/wayland/wayland-protocols/-/archive/0091197f5c1b1f2c131f1410e99f9c95d50646be/wayland-protocols-0091197f5c1b1f2c131f1410e99f9c95d50646be.tar.gz?path=staging",
        "https://gitlab.freedesktop.org/wayland/wayland-protocols/-/archive/0091197f5c1b1f2c131f1410e99f9c95d50646be/wayland-protocols-0091197f5c1b1f2c131f1410e99f9c95d50646be.tar.gz?path=unstable",
        "https://gitlab.freedesktop.org/wayland/wayland-protocols/-/archive/0091197f5c1b1f2c131f1410e99f9c95d50646be/wayland-protocols-0091197f5c1b1f2c131f1410e99f9c95d50646be.tar.gz?path=experimental",
    ];
    let blacklist = [
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
        .get(waylalnd_xml_path)
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
            if let Some(ext) = path.extension() {
                if let Some(name) = path.file_name() {
                    if ext == "xml"
                        && !blacklist.contains(&name.to_string_lossy().into_owned().as_str())
                    {
                        unwrapped.unpack(protocols_path.join(name)).unwrap();
                    }
                }
            }
        }
    }
}
