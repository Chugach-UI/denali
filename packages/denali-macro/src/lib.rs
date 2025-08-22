mod protocol_parser;
mod wire;
mod helper;

use std::{ffi::OsString, fs::File, path::PathBuf};

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use walkdir::WalkDir;

#[proc_macro]
pub fn wayland_protocols(input: TokenStream) -> TokenStream {
    let expr = syn::parse_macro_input!(input as syn::LitStr);

    match gen_protocols_inner(expr) {
        Ok(stream) => stream,
        Err(err) => quote! {
            compile_error!("Failed to generate Wayland protocol: {err}", err = #err);
        }
        .into(),
    }
}

fn gen_protocols_inner(expr: syn::LitStr) -> Result<TokenStream, String> {
    let path: OsString = expr.value().into();
    let path = if let Some(manifest_dir) = std::env::var_os("CARGO_MANIFEST_DIR") {
        let mut buf = PathBuf::from(manifest_dir);
        buf.push(path);
        buf
    } else {
        path.into()
    };

    let protocols = collect_files(&path)?.into_iter().map(|file| 
        protocol_parser::parse_protocol(file)
            .map_err(|_| "Failed to parse Wayland protocol file")
    ).filter_map(Result::ok);

    let modules = protocols.map(|protocol| {
        let mod_name = format_ident!("{}", protocol.name.to_case(Case::Snake));
        let desc = helper::description_to_docstring(&protocol.description);

        let events = protocol.interfaces.iter().flat_map(|interface| {
            interface.elements.iter().filter_map(|element| {
                if let protocol_parser::Element::Event(event) = element {
                    Some(wire::build_event(event))
                } else {
                    None
                }
            })
        });

        quote! {
            #desc
            pub mod #mod_name {
                #(#events)*
            }
        }
    });

    Ok(quote! {
        #(#modules)*
    }
    .into())
}

fn collect_files(path: &PathBuf) -> Result<Vec<File>, String> {
    let mut files = Vec::<File>::new();
    if path.is_file() {
        let file = File::open(&path).map_err(|_| "Failed to read Wayland protocol file: {}")?;
        files.push(file);
    } else if path.is_dir() {
        for path in WalkDir::new(path)
            .into_iter()
            .filter_map(Result::ok)
            .map(|e| e.into_path())
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
