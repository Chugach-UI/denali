use std::collections::BTreeMap;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    build_ident, helpers::{arg_type_to_rust_type, build_documentation, expand_argument_type}, protocol_parser::{Arg, Description, Event, Request}
};

pub fn build_event(event: &Event, interface_map: &BTreeMap<String, String>) -> TokenStream {
    let message = Message::Event(event);
    build_message(&message, interface_map)
}
pub fn build_request(request: &Request, interface_map: &BTreeMap<String, String>) -> TokenStream {
    let message = Message::Request(request);
    build_message(&message, interface_map)
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

    fn description(&self) -> &Option<Description> {
        match self {
            Message::Event(event) => &event.description,
            Message::Request(request) => &request.description,
        }
    }

    fn since(&self) -> &Option<String> {
        match self {
            Message::Event(event) => &event.since,
            Message::Request(request) => &request.since,
        }
    }

    fn deprecated_since(&self) -> &Option<String> {
        match self {
            Message::Event(event) => &event.deprecated_since,
            Message::Request(request) => &request.deprecated_since,
        }
    }

    fn args(&self) -> &[Arg] {
        match self {
            Message::Event(event) => &event.args,
            Message::Request(request) => &request.args,
        }
    }

    fn is_request(&self) -> bool {
        matches!(self, Message::Request(_))
    }
}

fn build_message(event: &Message, interface_map: &BTreeMap<String, String>) -> TokenStream {
    let suffix = if event.is_request() {
        "Request"
    } else {
        "Event"
    };
    let name = format_ident!("{}{suffix}", event.name().to_case(Case::Pascal));
    let docs = build_documentation(
        event.description(),
        &None,
        event.since(),
        event.deprecated_since(),
    );

    let arg_names = event
        .args()
        .iter()
        .map(|arg| {
            build_ident(&arg.name, Case::Snake)
        })
        .collect::<Vec<_>>();

    let struct_members = event
        .args()
        .iter()
        .map(|arg| {
            let arg_name = build_ident(&arg.name, Case::Snake);
            let arg_docs = build_documentation(&arg.description, &arg.summary, &None, &None);
            let arg_type = expand_argument_type(arg, interface_map, Some("'a"));
            quote! {
                #arg_docs
                pub #arg_name: #arg_type,
            }
        })
        .collect::<Vec<_>>();

    let lifetime = event
        .args()
        .iter()
        .find(|arg| arg.type_ == "string" || arg.type_ == "array")
        .map(|_| quote! { 'a })
        .into_iter()
        .collect::<Vec<_>>();

    let args_with_size = event
        .args()
        .iter()
        .filter(|arg| arg.type_ != "fd")
        .collect::<Vec<_>>();

    let compile_time_size = if args_with_size
        .iter()
        .any(|arg| arg.type_ == "string" || arg.type_ == "array")
    {
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
           impl #(<#lifetime>)* denali_utils::wire::serde::CompileTimeMessageSize for #name #(<#lifetime>)* {
               const SIZE: usize = #size;
           }
        }
    };

    quote! {
        #docs
        pub struct #name #(<#lifetime>)* {
            #(#struct_members)*
        }
        impl #(<#lifetime>)* denali_utils::wire::serde::MessageSize for #name #(<#lifetime>)* {
            fn size(&self) -> usize {
                let mut size = 0;
                #(
                    size += self.#arg_names.size();
                )*
                size
            }
        }
        #compile_time_size
        impl #(<#lifetime>)* denali_utils::wire::serde::Decode for #name #(<#lifetime>)* {
            fn decode(data: &[u8]) -> Result<Self, denali_utils::wire::serde::SerdeError> {
                let mut traverser = denali_utils::wire::MessageDecoder::new(data);

                #(
                    let #arg_names = traverser.read()?;
                )*

                Ok(Self {
                    #(#arg_names),*
                })
            }
        }
        impl #(<#lifetime>)* denali_utils::wire::serde::Encode for #name #(<#lifetime>)* {
            fn encode(&self, data: &mut [u8]) -> Result<usize, denali_utils::wire::serde::SerdeError> {
                let mut traverser = denali_utils::wire::MessageEncoder::new(data);

                #(
                    traverser.write(&self.#arg_names)?;
                )*

                Ok(traverser.position() as usize)
            }
        }
    }
}
