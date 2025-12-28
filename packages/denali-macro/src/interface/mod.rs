// mod method;

use std::collections::BTreeMap;

use convert_case::{Boundary, Case, Casing};
use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    build_ident,
    helpers::build_documentation,
    protocol_parser::{ArgType, Interface, InterfaceElement, Message},
    wire::build_enum,
};

fn event_needs_lifetime(event: &Message) -> bool {
    event.args.iter().any(|arg| {
        matches!(
            arg.arg_type,
            ArgType::String | ArgType::Array | ArgType::GenericNewId
        )
    })
}

fn build_event_enum(interface: &Interface, events: &[Message]) -> TokenStream {
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
                use denali_core::Interface;
                if interface != #interface_ident::INTERFACE {
                    return Err(denali_core::handler::DecodeMessageError::UnknownInterface(interface.to_string()));
                }

                match opcode {
                    #(#try_decode_opcode_arms)*
                    _ => Err(denali_core::handler::DecodeMessageError::UnknownOpcode(opcode)),
                }
            }
        }
        impl #lifetime denali_core::handler::MessageTarget for #name #lifetime {
            type Target = #interface_ident;
        }
    }
}

pub fn build_interface(
    interface: &Interface,
    interface_map: &BTreeMap<String, String>,
) -> TokenStream {
    let documentation = build_documentation(Some(&interface.description), None, None, None);
    let interface_str = interface
        .name
        .without_boundaries(&[Boundary::LOWER_DIGIT])
        .to_case(Case::Snake);
    let name = build_ident(&interface.name, Case::Pascal);
    let version = interface.version;

    quote! {
        #documentation
        pub struct #name(());

        impl denali_core::Interface for #name {
            const INTERFACE: &'static str = #interface_str;

            const MAX_VERSION: u32 = #version;

            type Event = Event;
            type Request = Request;
        }
    }
}

pub fn build_interface_module(
    interface: &Interface,
    interface_map: &BTreeMap<String, String>,
) -> TokenStream {
    let interface_name = build_ident(&interface.name, Case::Snake);
    let interface_desc = build_documentation(Some(&interface.description), None, None, None);
    let interface_version = interface.version;

    let type_name = build_ident(&interface.name, Case::Pascal);

    let enums = interface.elements.iter().filter_map(|element| {
        if let InterfaceElement::Enum(enum_) = element {
            Some(build_enum(enum_))
        } else {
            None
        }
    });

    let interface = build_interface(interface, interface_map);

    quote! {
        #interface_desc
        pub mod #interface_name {
            pub const VERSION: u32 = #interface_version;

            #interface

            pub struct Request;
            impl denali_core::message::IncomingMessage<denali_core::message::Request> for Request {
                type Interface = #type_name;
                fn try_decode(
                    interface: &str,
                    opcode: u16,
                    message_type: denali_core::message::MessageType,
                    data: &[u8],
                ) -> Result<Self, denali_core::message::DecodeMessageError> {
                    todo!()
                }
            }
            pub struct Event;
            impl denali_core::message::IncomingMessage<denali_core::message::Event> for Event {
                type Interface = #type_name;
                fn try_decode(
                    interface: &str,
                    opcode: u16,
                    message_type: denali_core::message::MessageType,
                    data: &[u8],
                ) -> Result<Self, denali_core::message::DecodeMessageError> {
                    todo!()
                }
            }

            #(#enums)*
        }
    }
}
