#![allow(missing_docs)]

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
    let wlr_protocols_commit = lock_lines[2].replace("wlr-protocols=", "");

    let wayland_xml_path = format!(
        "https://gitlab.freedesktop.org/wayland/wayland/-/raw/{wayland_commit}/protocol/wayland.xml"
    );
    let wayland_protocols_archive_paths: Vec<String> = ["stable", "staging", "unstable", "experimental"]
        .iter()
        .map(|path| {
            format!(
                "https://gitlab.freedesktop.org/wayland/wayland-protocols/-/archive/{wayland_protocols_commit}/wayland-protocols-{wayland_protocols_commit}.tar.gz?path={path}",
            )
        })
        .collect();
    let wlr_protocols_unstable_archive_path = format!(
        "https://gitlab.freedesktop.org/wlroots/wlr-protocols/-/archive/{wlr_protocols_commit}/wlr-protocols-{wlr_protocols_commit}.tar.gz?path=unstable",
    );

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let protocols_path = Path::new(&out_dir).join("protocols");
    fs::create_dir_all(&protocols_path).unwrap();

    let client = Client::new();

    get_file(&client, protocols_path.as_path(), wayland_xml_path);

    for archive_path in wayland_protocols_archive_paths {
        unpack_protocols_tar(&client, protocols_path.as_path(), archive_path);
    }

    unpack_protocols_tar(
        &client,
        protocols_path.as_path(),
        wlr_protocols_unstable_archive_path,
    );

    let client_code_path = Path::new(&out_dir).join("denali_client_protocols.rs");
    fs::write(
        client_code_path,
        format!(
            "denali_macro::wayland_protocols!(\"{}\");",
            protocols_path.to_string_lossy()
        ),
    )
    .unwrap();

    let server_code_path = Path::new(&out_dir).join("denali_server_protocols.rs");
    fs::write(server_code_path, "pub mod todo {}\n").unwrap();
}

fn get_file(client: &Client, protocols_path: &Path, file_path: String) {
    let bytes = client.get(file_path).send().unwrap().bytes().unwrap();

    fs::write(protocols_path.join("wayland.xml"), bytes).unwrap();
}

fn unpack_protocols_tar(client: &Client, protocols_path: &Path, archive_path: String) {
    let protocol_blacklist = [
        "linux-dmabuf-unstable-v1.xml",
        "tablet-unstable-v1.xml",
        "tablet-unstable-v2.xml",
        "text-input-unstable-v1.xml",
        "xdg-foreign-unstable-v1.xml",
        "xdg-shell-unstable-v5.xml",
        "xdg-shell-unstable-v6.xml",
    ];

    let bytes = client.get(archive_path).send().unwrap().bytes().unwrap();

    let tar = GzDecoder::new(&bytes[..]);
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
