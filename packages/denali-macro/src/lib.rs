mod protocol_parser;

use std::{ffi::OsString, fs::File, path::PathBuf};

use proc_macro::TokenStream;
use quote::quote;
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

    let files = collect_files(&path)?;

    let mut message = String::new();
    for file in files {
        let protocol = protocol_parser::parse_protocol(file)
            .map_err(|_| "Failed to parse Wayland protocol file")?;
        message.push_str(format!("Parsed protocol: {}\n", protocol.name).as_str());
    }

    Ok(quote! {
        compile_error!(#message);
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
