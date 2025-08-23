use std::collections::BTreeMap;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::quote;

use crate::{build_ident, helpers::build_documentation, protocol_parser::{Element, Interface}, wire::{build_enum, build_event, build_request}};

pub fn build_interface(interface: &Interface) -> TokenStream {
    let documentation = build_documentation(&interface.description, &None, &None, &None);
    let interface_str = interface.name.to_case(Case::Snake);
    let name = build_ident(&interface.name, Case::Pascal);
    let version = interface.version;

    quote! {
        #documentation
        pub struct #name(denali_utils::proxy::Proxy);
        impl denali_utils::Interface for #name {
            const INTERFACE: &'static str = #interface_str;

            const MAX_VERSION: u32 = #version;
        }
    }
}

pub fn build_interface_module(interface: &Interface, interface_map: &BTreeMap<String, String>) -> TokenStream {
    let interface_name = build_ident(&interface.name, Case::Snake);
    let interface_desc =
        build_documentation(&interface.description, &None, &None, &None);
    let interface_version = interface.version;

    let events = interface.elements.iter().map(|element| match element {
        Element::Event(event) => {
            Some(build_event(event, interface_map))
        }
        Element::Request(request) => {
            Some(build_request(request, interface_map))
        }
        Element::Enum(enum_) => Some(build_enum(enum_)),
    });

    let interface = build_interface(interface);

    quote! {
        #interface_desc
        pub mod #interface_name {
            pub const VERSION: u32 = #interface_version;

            #interface

            #(#events)*
        }
    }
}