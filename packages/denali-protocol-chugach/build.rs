#![allow(missing_docs)]

use std::env;
use std::fs;
use std::path::Path;

use denali_build::export_dependency_info;
use denali_build::parse_dependency_info;
use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use tar::Archive;

const LOCK: &str = "main";

pub fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let chugach_archive_path =
        format!("https://github.com/Chugach-UI/cascade-protocols/archive/{LOCK}.tar.gz");

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let protocols_path = Path::new(&out_dir).join("protocols");
    fs::create_dir_all(&protocols_path).unwrap();

    let client = Client::new();

    unpack_protocols_tar(&client, protocols_path.as_path(), chugach_archive_path);

    let map = denali_build::generate_protocol_map(protocols_path.as_path());
    export_dependency_info(&map);

    let base_dep_info = parse_dependency_info("denali_protocol_base");

    let code_path = Path::new(&out_dir).join("protocols.rs");
    fs::write(
        code_path,
        denali_build::denali_macro_invocations_with_deps(
            &protocols_path.to_string_lossy(),
            &[("denali_protocol_base", &base_dep_info)],
        ),
    )
    .unwrap();
}

fn unpack_protocols_tar(client: &Client, protocols_path: &Path, archive_path: String) {
    let bytes = client.get(archive_path).send().unwrap().bytes().unwrap();

    let tar = GzDecoder::new(&bytes[..]);
    let mut archive = Archive::new(tar);

    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let path = entry.path().unwrap();
        if let Some(ext) = path.extension()
            && let Some(name) = path.file_name()
            && ext == "xml"
        {
            entry.unpack(protocols_path.join(name)).unwrap();
        }
    }
}
