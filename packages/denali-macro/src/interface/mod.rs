mod method;

use std::collections::BTreeMap;

use convert_case::{Boundary, Case, Casing};
use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    build_ident,
    helpers::build_documentation,
    interface::method::build_request_method,
    protocol_parser::{Element, Event, Interface},
    wire::{build_enum, build_event, build_request},
};

fn event_needs_lifetime(event: &Event) -> bool {
    event.args.iter().any(|arg| {
        matches!(arg.type_.as_str(), "string" | "array")
            || (arg.type_ == "new_id" && arg.interface.is_none())
    })
}

fn build_event_enum(interface: &Interface, events: &[Event]) -> TokenStream {
    let needs_lifetime = events.iter().any(event_needs_lifetime);

    let lifetime = if needs_lifetime {
        quote! { <'a> }
    } else {
        quote! {}
    };

    let variants = events.iter().map(|event| {
        let variant_ident = build_ident(&event.name, Case::Pascal);
        let event_struct_name = build_ident(&format!("{}Event", event.name), Case::Pascal);
        let event_struct_name = if event_needs_lifetime(event) {
            quote! {#event_struct_name<'a>}
        } else {
            quote! {#event_struct_name}
        };

        quote! {
            #variant_ident(#event_struct_name)
        }
    });
    let try_decode_opcode_arms = events.iter().enumerate().map(|(i, event)| {
        let variant_ident = build_ident(&event.name, Case::Pascal);
        let event_struct_name = build_ident(&format!("{}Event", event.name), Case::Pascal);

        let opcode = i as u16;

        quote! {
            #opcode => #event_struct_name::decode(data).map(Self::#variant_ident).map_err(Into::into),
        }
    });

    let name = build_ident(&format!("{}Event", interface.name), Case::Pascal);
    let interface_ident = build_ident(&interface.name, Case::Pascal);

    quote! {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum #name #lifetime {
            #(#variants),*
        }
        impl #lifetime denali_core::handler::Message for #name #lifetime {
            fn try_decode(interface: &str, opcode: u16, data: &[u8]) -> Result<Self, denali_core::handler::DecodeMessageError> {
                use denali_core::wire::serde::Decode;
                use denali_client_core::Interface;
                if interface != #interface_ident::INTERFACE {
                    return Err(denali_core::handler::DecodeMessageError::UnknownInterface(interface.to_string()));
                }

                match opcode {
                    #(#try_decode_opcode_arms)*
                    _ => Err(denali_core::handler::DecodeMessageError::UnknownOpcode(opcode)),
                }
            }
        }
    }
}

//TODO: DO SERVER SIDE CODEGEN AS WELL
pub fn build_interface(
    interface: &Interface,
    interface_map: &BTreeMap<String, String>,
) -> TokenStream {
    let documentation = build_documentation(interface.description.as_ref(), None, None, None);
    let interface_str = interface
        .name
        .without_boundaries(&[Boundary::LOWER_DIGIT])
        .to_case(Case::Snake);
    let name = build_ident(&interface.name, Case::Pascal);
    let version = interface.version;

    let methods = interface.elements.iter().filter_map(|element| {
        if let Element::Request(request) = element {
            Some(build_request_method(request, interface_map))
        } else {
            None
        }
    });

    let events = interface
        .elements
        .iter()
        .cloned()
        .filter_map(|element| {
            if let Element::Event(event) = element {
                Some(event)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let event_enum = build_event_enum(interface, &events);

    quote! {
        #documentation
        pub struct #name(denali_client_core::proxy::Proxy);

        impl #name {
            #(#methods)*
        }

        impl From<denali_client_core::proxy::Proxy> for #name {
            fn from(proxy: denali_client_core::proxy::Proxy) -> Self {
                Self(proxy)
            }
        }

        impl denali_client_core::Object for #name {
            fn id(&self) -> u32 {
                self.0.id()
            }
            fn send_request(&self, request: denali_client_core::proxy::RequestMessage) {
                self.0.send_request(request);
            }
        }
        impl denali_client_core::Interface for #name {
            const INTERFACE: &'static str = #interface_str;

            const MAX_VERSION: u32 = #version;
        }

        #event_enum
    }
}

pub fn build_interface_module(
    interface: &Interface,
    interface_map: &BTreeMap<String, String>,
) -> TokenStream {
    let interface_name = build_ident(&interface.name, Case::Snake);
    let interface_desc = build_documentation(interface.description.as_ref(), None, None, None);
    let interface_version = interface.version;

    let events = interface.elements.iter().map(|element| match element {
        Element::Event(event) => Some(build_event(event, interface, interface_map)),
        Element::Request(request) => Some(build_request(request, interface, interface_map)),
        Element::Enum(enum_) => Some(build_enum(enum_)),
    });

    let interface = build_interface(interface, interface_map);

    quote! {
        #interface_desc
        pub mod #interface_name {
            pub const VERSION: u32 = #interface_version;

            #interface

            #(#events)*
        }
    }
}
