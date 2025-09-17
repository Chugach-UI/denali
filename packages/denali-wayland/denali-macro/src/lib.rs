#![allow(missing_docs)]

mod helpers;
mod interface;
mod protocol;
mod protocol_parser;
mod wire;

use std::{collections::BTreeMap, ffi::OsString, fs::File, path::PathBuf};

use helpers::build_ident;
use proc_macro::TokenStream;
use protocol::build_protocol;
use protocol_parser::Protocol;
use quote::quote;
use walkdir::WalkDir;

#[proc_macro]
pub fn wayland_protocols(input: TokenStream) -> TokenStream {
    let expr = syn::parse_macro_input!(input as syn::LitStr);

    match gen_protocols_inner(&expr) {
        Ok(stream) => stream,
        Err(err) => quote! {
            compile_error!("Failed to generate Wayland protocol: {err}", err = #err);
        }
        .into(),
    }
}

fn gen_protocols_inner(expr: &syn::LitStr) -> Result<TokenStream, String> {
    let path: OsString = expr.value().into();
    let path = if let Some(manifest_dir) = std::env::var_os("CARGO_MANIFEST_DIR") {
        let mut buf = PathBuf::from(manifest_dir);
        buf.push(path);
        buf
    } else {
        path.into()
    };

    let protocols = collect_files(&path)?
        .into_iter()
        .map(|file| {
            protocol_parser::parse_protocol(file)
                .map_err(|_| "Failed to parse Wayland protocol file")
        })
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    let interface_map = build_interface_map(&protocols);

    let protocols = protocols
        .into_iter()
        .map(|protocol| build_protocol(&protocol, &interface_map));

    Ok(quote! {
        #(#protocols)*
    }
    .into())
}

fn collect_files(path: &PathBuf) -> Result<Vec<File>, String> {
    let mut files = Vec::<File>::new();
    if path.is_file() {
        let file = File::open(path).map_err(|_| "Failed to read Wayland protocol file: {}")?;
        files.push(file);
    } else if path.is_dir() {
        for path in WalkDir::new(path)
            .into_iter()
            .filter_map(Result::ok)
            .map(walkdir::DirEntry::into_path)
            .filter(|p| p.is_file() && p.extension().is_some_and(|ext| ext == "xml"))
        {
            let file = File::open(&path).map_err(|_| "Failed to read Wayland protocol file: {}")?;
            files.push(file);
        }
    } else {
        return Err("Expected path to be a file or directory".to_string());
    }

    Ok(files)
}

/// Builds a map of interface to its protocol
fn build_interface_map(protocols: &[Protocol]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();

    for protocol in protocols {
        for interface in &protocol.interfaces {
            map.insert(interface.name.clone(), protocol.name.clone());
        }
    }

    map
}
