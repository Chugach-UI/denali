use std::collections::BTreeMap;

use convert_case::Case;
use proc_macro2::TokenStream;

use crate::{
    Protocol, build_ident, helpers::build_documentation, interface::build_interface_module,
};
use quote::quote;

pub fn build_protocol(
    protocol: &Protocol,
    interface_map: &BTreeMap<String, String>,
) -> TokenStream {
    let mod_name = build_ident(&protocol.name, Case::Snake);

    let desc = build_documentation(protocol.description.as_ref(), None, None, None);

    let interfaces = protocol
        .interfaces
        .iter()
        .map(|interface| build_interface_module(interface, interface_map));

    quote! {
        #desc
        #[allow(deprecated)]
        pub mod #mod_name {
            #(#interfaces)*
        }
    }
}
