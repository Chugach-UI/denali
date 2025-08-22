mod protocol_parser;

use std::{ffi::OsString, fs::File, path::PathBuf};

use proc_macro::TokenStream;
use quote::quote;

#[proc_macro]
pub fn wayland_protocol(input: TokenStream) -> TokenStream {
    let expr = syn::parse_macro_input!(input as syn::LitStr);

    match gen_protocol_inner(expr) {
        Ok(stream) => stream,
        Err(err) => quote! {
            compile_error!("Failed to generate Wayland protocol: {err}", err = #err);
        }
        .into(),
    }
}

fn gen_protocol_inner(expr: syn::LitStr) -> Result<TokenStream, String> {
    let path: OsString = expr.value().into();
    let path = if let Some(manifest_dir) = std::env::var_os("CARGO_MANIFEST_DIR") {
        let mut buf = PathBuf::from(manifest_dir);
        buf.push(path);
        buf
    } else {
        path.into()
    };

    let file = File::open(&path).map_err(|_| "Failed to read Wayland protocol file: {}")?;
    let protocol = protocol_parser::parse_protocol(file).map_err(|_| "Failed to parse Wayland protocol file")?;
    let message = format!("Parsed protocol: {}", protocol.name);

    Ok(quote! {
        compile_error!(#message);
    }
    .into())
}
