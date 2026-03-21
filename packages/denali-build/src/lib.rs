//! Build script helpers for writing Denali protocol crates

use std::{
    fs::{File, read_dir},
    io::Read,
    path::Path,
};

use convert_case::Casing;
use denali_protocol_parser::parse_protocol;
use itertools::Itertools;

fn parse_pair(pair: &str) -> (&str, &str) {
    println!("{pair}");
    let (key, value) = pair.split_once(',').unwrap();

    let key = key.trim().trim_matches(['(', ')', '"']);
    let value = value.trim().trim_matches(['(', ')', '"']);

    (key, value)
}

fn parse_info(info: &str) -> Vec<(&str, &str)> {
    info.trim_matches(['[', ']'])
        .rsplit('(')
        .take_while(|pair| pair.contains(')'))
        .map(|pair| parse_pair(pair.trim().trim_end_matches(",")))
        .collect()
}

/// Parses dependency info from a crate using build script metadata
///
/// Dependency info is saved in the format [("<interface>", "<protocol>")]
pub fn parse_dependency_info(crate_name: &str) -> Vec<(String, String)> {
    let key = crate_name.to_case(convert_case::Case::UpperSnake);
    let key = format!("{key}_DEPENDENCY_INFO");
    let value = std::env::var(key).unwrap();

    parse_info(&value)
        .iter()
        .map(|pair| (pair.0.to_string(), pair.1.to_string()))
        .collect()
}

pub fn generate_protocol_map(directory: &Path) -> Vec<(String, String)> {
    let mut map = Vec::new();
    for entry in read_dir(directory).unwrap() {
        let protocol = parse_protocol(File::open(entry.unwrap().path()).unwrap()).unwrap();
        let sub_map = protocol
            .interfaces
            .into_iter()
            .map(|interface| (interface.name, protocol.name.clone()));

        map.extend(sub_map)
    }
    map
}

pub fn export_dependency_info(map: &[(impl AsRef<str>, impl AsRef<str>)]) {
    let pairs = map
        .iter()
        .map(|pair| format!("(\"{}\", \"{}\")", pair.0.as_ref(), pair.1.as_ref()))
        .join(",");

    let info = format!("[{pairs}]");

    println!("cargo:DEPENDENCY_INFO={info}")
}

fn external_interface_map(crate_name: &str, map: &[(impl AsRef<str>, impl AsRef<str>)]) -> String {
    let pairs = map
        .iter()
        .map(|(iface, protocol)| format!("(\"{}\", \"{}\")", iface.as_ref(), protocol.as_ref()))
        .join(",");

    format!("[\"{crate_name}\", {pairs}]")
}

/// Generate external interface maps for a protocols macro invocation
pub fn external_interface_maps(maps: &[(&str, &[(&str, &str)])]) -> String {
    let elems = maps
        .iter()
        .map(|(cn, map)| external_interface_map(cn, map))
        .join(",");

    format!("[{elems}]")
}

/// Generate the contents of a file exporting wayland protocol(s)
pub fn denali_macro_invocations(protocol_path: &str, maps: &[(&str, &[(&str, &str)])]) -> String {
    let maps = external_interface_maps(maps);
    format!("denali_macro::wayland_protocols!(\"{protocol_path}\", {maps});",)
}

#[cfg(test)]
mod tests {
    #[test]
    fn dependency_info_parse() {
        let dep_info = "[(\"wl_buffer\", \"wayland\"), (\"wl_buffer\", \"wayland\")]";
        let info = super::parse_info(dep_info);

        assert_eq!(info[0], ("wl_buffer", "wayland"))
    }
}
