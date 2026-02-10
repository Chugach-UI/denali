//! Build script helpers for writing Denali protocol crates

use itertools::Itertools;

fn external_interface_map(crate_name: &str, map: &[(&str, &str)]) -> String {
    let pairs = map
        .iter()
        .map(|(iface, protocol)| format!("(\"{iface}\", \"{protocol}\")"))
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
    format!(
        "denali_macro::wayland_protocols!(\"{protocol_path}\", {maps});\ndenali_macro::dependency_info!(\"{protocol_path}\");",
    )
}
