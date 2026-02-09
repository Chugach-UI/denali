#![allow(missing_docs)]

mod helpers;
mod interface;
mod protocol;
mod protocol_parser;
mod type_paths;
mod wire;

use std::{collections::BTreeMap, ffi::OsString, fs::File, path::PathBuf};

use helpers::build_ident;
use proc_macro::TokenStream;
use protocol::build_protocol;
use protocol_parser::Protocol;
use quote::quote;
use syn::{LitStr, Token, parse::Parse};
use walkdir::WalkDir;

use crate::type_paths::InterfaceMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessageType {
    Request,
    Event,
}

struct InterfaceMapPair {
    interface: LitStr,
    protocol: LitStr,
}
impl Parse for InterfaceMapPair {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        syn::parenthesized!(content in input);

        let interface: LitStr = content.parse()?;
        content.parse::<Token![,]>()?;
        let protocol: LitStr = content.parse()?;

        Ok(Self {
            interface,
            protocol,
        })
    }
}

struct InterfaceMapInput {
    crate_name: LitStr,
    items: syn::punctuated::Punctuated<InterfaceMapPair, syn::Token![,]>,
}
impl Parse for InterfaceMapInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        syn::bracketed!(content in input);

        let crate_name: LitStr = content.parse()?;
        content.parse::<Token![,]>()?;
        let items = content.parse_terminated(InterfaceMapPair::parse, Token![,])?;
        Ok(Self { crate_name, items })
    }
}

struct InterfaceMapsInput {
    lists: syn::punctuated::Punctuated<InterfaceMapInput, syn::Token![,]>,
}
impl Parse for InterfaceMapsInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        syn::bracketed!(content in input);

        let lists = content.parse_terminated(InterfaceMapInput::parse, Token![,])?;
        Ok(Self { lists })
    }
}

struct ProtocolInput {
    protocol_path: LitStr,
    external_interface_maps: InterfaceMapsInput,
}

impl Parse for ProtocolInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let protocol_path: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let external_interface_maps: InterfaceMapsInput = input.parse()?;

        Ok(Self {
            protocol_path,
            external_interface_maps,
        })
    }
}

#[proc_macro]
pub fn wayland_protocols(input: TokenStream) -> TokenStream {
    let expr = syn::parse_macro_input!(input as ProtocolInput);

    match gen_protocols_inner(expr) {
        Ok(stream) => stream,
        Err(err) => quote! {
            compile_error!("Failed to generate Wayland protocol: {err}", err = #err);
        }
        .into(),
    }
}

#[proc_macro]
pub fn dependency_info(input: TokenStream) -> TokenStream {
    let expr = syn::parse_macro_input!(input as syn::LitStr);

    let protocols = protocols_from_path(&expr).unwrap();

    let pairs = protocols
        .iter()
        .map(|protocol| {
            protocol.interfaces.iter().map(|interface| {
                let name = &interface.name;
                let protocol = &protocol.name;
                quote! {(#name, #protocol)}
            })
        })
        .flatten()
        .collect::<Vec<_>>();

    let len = pairs.len();

    quote! {
        pub const DEPENDENCY_INFO: [(&str, &str); #len] = [
            #(#pairs),*
        ];
    }
    .into()
}

fn protocols_from_path(path: &syn::LitStr) -> Result<Vec<Protocol>, String> {
    let path: OsString = path.value().into();
    let path = if let Some(manifest_dir) = std::env::var_os("CARGO_MANIFEST_DIR") {
        let mut buf = PathBuf::from(manifest_dir);
        buf.push(path);
        buf
    } else {
        path.into()
    };

    Ok(collect_files(&path)?
        .into_iter()
        .map(|file| {
            protocol_parser::parse_protocol(file)
                .map_err(|_| "Failed to parse Wayland protocol file")
        })
        .filter_map(Result::ok)
        .collect::<Vec<_>>())
}

fn gen_protocols_inner(input: ProtocolInput) -> Result<TokenStream, String> {
    let protocols = protocols_from_path(&input.protocol_path)?;

    let mut interface_map = InterfaceMap::with_local_protocols(&protocols);

    for map in input.external_interface_maps.lists {
        interface_map.add_external_protocols(
            &map.crate_name.value(),
            map.items
                .iter()
                .map(|pair| (pair.interface.value(), pair.protocol.value()))
                .collect(),
        );
    }

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
