use std::collections::BTreeMap;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    build_ident,
    helpers::{
        arg_type_to_rust_type, build_documentation, expand_argument_type,
        is_size_known_at_compile_time,
    },
    protocol_parser::{Arg, Description, Event, Interface, Request},
};

pub fn build_event(
    event: &Event,
    interface: &Interface,
    interface_map: &BTreeMap<String, String>,
) -> TokenStream {
    let message = Message::Event(event);
    build_message(&message, interface, interface_map)
}
pub fn build_request(
    request: &Request,
    interface: &Interface,
    interface_map: &BTreeMap<String, String>,
) -> TokenStream {
    let message = Message::Request(request);
    build_message(&message, interface, interface_map)
}

enum Message<'a> {
    Event(&'a Event),
    Request(&'a Request),
}
impl Message<'_> {
    fn name(&self) -> &str {
        match self {
            Message::Event(event) => &event.name,
            Message::Request(request) => &request.name,
        }
    }

    const fn description(&self) -> Option<&Description> {
        match self {
            Message::Event(event) => event.description.as_ref(),
            Message::Request(request) => request.description.as_ref(),
        }
    }

    const fn since(&self) -> Option<&String> {
        match self {
            Message::Event(event) => event.since.as_ref(),
            Message::Request(request) => request.since.as_ref(),
        }
    }

    const fn deprecated_since(&self) -> Option<&String> {
        match self {
            Message::Event(event) => event.deprecated_since.as_ref(),
            Message::Request(request) => request.deprecated_since.as_ref(),
        }
    }

    fn args(&self) -> &[Arg] {
        match self {
            Message::Event(event) => &event.args,
            Message::Request(request) => &request.args,
        }
    }

    const fn is_request(&self) -> bool {
        matches!(self, Message::Request(_))
    }
}

#[allow(clippy::too_many_lines)]
fn build_message(
    message: &Message<'_>,
    interface: &Interface,
    interface_map: &BTreeMap<String, String>,
) -> TokenStream {
    let suffix = if message.is_request() {
        "Request"
    } else {
        "Event"
    };

    let mut opcode: u16 = 0;
    for elem in &interface.elements {
        match elem {
            crate::protocol_parser::Element::Request(req) if message.is_request() => {
                if req.name == message.name() {
                    break;
                }
                opcode += 1;
            }
            crate::protocol_parser::Element::Event(evt) if !message.is_request() => {
                if evt.name == message.name() {
                    break;
                }
                opcode += 1;
            }
            _ => {}
        }
    }
    let opcode = quote! { const OPCODE: u16 = #opcode; };

    let name = format_ident!("{}{suffix}", message.name().to_case(Case::Pascal));
    let docs = build_documentation(
        message.description(),
        None,
        message.since(),
        message.deprecated_since(),
    );

    let arg_names = message
        .args()
        .iter()
        .map(|arg| build_ident(&arg.name, Case::Snake))
        .collect::<Vec<_>>();

    let struct_members = message
        .args()
        .iter()
        .map(|arg| {
            let arg_name = build_ident(&arg.name, Case::Snake);
            let arg_docs = build_documentation(arg.description.as_ref(), arg.summary.as_ref(), None, None);
            let arg_type = expand_argument_type(arg, interface_map, Some("'a"));
            quote! {
                #arg_docs
                pub #arg_name: #arg_type,
            }
        })
        .collect::<Vec<_>>();

    let lifetime = message
        .args()
        .iter()
        .find(|arg| {
            arg.type_ == "string"
                || arg.type_ == "array"
                || (arg.type_ == "new_id" && arg.interface.is_none())
        })
        .map(|_| quote! { 'a })
        .into_iter()
        .collect::<Vec<_>>();

    let args_with_size = message
        .args()
        .iter()
        .filter(|arg| arg.type_ != "fd")
        .collect::<Vec<_>>();

    let compile_time_size = if is_size_known_at_compile_time(&args_with_size) {
        quote! {}
    } else {
        let size = if args_with_size.is_empty() {
            quote! { 0 }
        } else {
            let arg_types_with_size = args_with_size
                .iter()
                .map(|arg| arg_type_to_rust_type(&arg.type_, None))
                .collect::<Vec<_>>();

            quote! { #(#arg_types_with_size::SIZE)+* }
        };
        quote! {
           impl #(<#lifetime>)* denali_core::wire::serde::CompileTimeMessageSize for #name #(<#lifetime>)* {
               const SIZE: usize = #size;
           }
        }
    };

    quote! {
        #docs
        pub struct #name #(<#lifetime>)* {
            #(#struct_members)*
        }
        impl #(<#lifetime>)* #name #(<#lifetime>)* {
            #opcode
        }
        impl #(<#lifetime>)* denali_core::wire::serde::MessageSize for #name #(<#lifetime>)* {
            fn size(&self) -> usize {
                let mut size = 0;
                #(
                    size += self.#arg_names.size();
                )*
                size
            }
        }
        #compile_time_size
        impl #(<#lifetime>)* denali_core::wire::serde::Decode for #name #(<#lifetime>)* {
            fn decode(data: &[u8]) -> Result<Self, denali_core::wire::serde::SerdeError> {
                let mut traverser = denali_core::wire::MessageDecoder::new(data);

                #(
                    let #arg_names = traverser.read()?;
                )*

                Ok(Self {
                    #(#arg_names),*
                })
            }
        }
        impl #(<#lifetime>)* denali_core::wire::serde::Encode for #name #(<#lifetime>)* {
            fn encode(&self, data: &mut [u8]) -> Result<usize, denali_core::wire::serde::SerdeError> {
                let mut traverser = denali_core::wire::MessageEncoder::new(data);

                #(
                    traverser.write(&self.#arg_names)?;
                )*

                Ok(traverser.position() as usize)
            }
        }
    }
}
