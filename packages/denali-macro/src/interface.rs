// mod method;

use std::collections::BTreeMap;

use convert_case::{Boundary, Case, Casing};
use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    MessageType, build_ident,
    helpers::{build_documentation, expand_argument_type, interface_path},
    protocol_parser::{ArgType, Interface, InterfaceElement, Message, MessageKind, NewIdType},
    wire::{build_enum, build_message_decode_body, build_message_encode_impl},
};

#[derive(Debug, Clone)]
pub struct InterfaceCtx {
    pub interface_name: syn::Ident,
    pub interface_str: String,

    pub event_enum_name: syn::Ident,
    pub event_enum_needs_lifetime: bool,
    pub request_enum_name: syn::Ident,
    pub request_enum_needs_lifetime: bool,
}

fn build_message_fields(
    ctx: &InterfaceCtx,
    interface_map: &BTreeMap<String, String>,
    message: &Message,
    outgoing: bool,
) -> (TokenStream, bool) {
    let interface_name = &ctx.interface_name;
    let vis = if outgoing {
        quote! { pub }
    } else {
        quote! {}
    };

    let mut needs_lifetime = false;

    let sender_field = if outgoing {
        if message.message_type == MessageKind::Destructor {
            quote! { #vis sender: ObjectId<#interface_name>, }
        } else {
            needs_lifetime = true;
            quote! { #vis sender: &'a ObjectId<#interface_name>, }
        }
    } else {
        quote! {}
    };

    let fields = message
        .args
        .iter()
        .filter(|field| {
            if outgoing && matches!(message.new_id_type, NewIdType::Typed { .. }) {
                !field.arg_type.is_new_id()
            } else {
                true
            }
        })
        .map(|arg| {
            let field_name = build_ident(&arg.name, Case::Snake);

            if outgoing && arg.arg_type == ArgType::GenericNewId {
                return quote! {
                    #field_name: denali_core::message::NewIdHint<I>
                };
            }

            let (field_type, uses_lifetime) =
                expand_argument_type(arg, interface_map, Some("'a"), outgoing);
            needs_lifetime |= uses_lifetime;

            quote! { #field_name: #field_type }
        });

    (
        quote! {
            {
                #sender_field
                #(#vis #fields),*
            }
        },
        needs_lifetime,
    )
}

fn build_incoming_message_enums(
    ctx: &mut InterfaceCtx,
    interface_map: &BTreeMap<String, String>,
    interface: &Interface,
    message_type: MessageType,
) -> TokenStream {
    let incoming_enum_name = if message_type == MessageType::Request {
        &ctx.request_enum_name
    } else {
        &ctx.event_enum_name
    };
    let interface_name = &ctx.interface_name;

    let messages = interface
        .elements
        .iter()
        .cloned()
        .filter_map(|elem| match elem {
            InterfaceElement::Request(message) if message_type == MessageType::Request => {
                Some(message)
            }
            InterfaceElement::Event(message) if message_type == MessageType::Event => Some(message),
            _ => None,
        })
        .collect::<Vec<_>>();

    let message_type_marker = if message_type == MessageType::Request {
        quote! { denali_core::message::Request }
    } else {
        quote! { denali_core::message::Event }
    };
    let message_type_enum = if message_type == MessageType::Request {
        quote! { denali_core::message::MessageType::Request }
    } else {
        quote! { denali_core::message::MessageType::Event }
    };

    let mut needs_lifetime = false;
    let variants = messages
        .iter()
        .map(|msg| {
            let event_name = build_ident(&msg.name, Case::Pascal);

            let (fields, uses_lifetime) = build_message_fields(ctx, interface_map, msg, false);

            needs_lifetime |= uses_lifetime;

            quote! { #event_name #fields }
        })
        .collect::<Vec<_>>();

    if message_type == MessageType::Request {
        ctx.request_enum_needs_lifetime = needs_lifetime;
    } else {
        ctx.event_enum_needs_lifetime = needs_lifetime;
    }

    let lifetime = if needs_lifetime {
        quote! { <'a> }
    } else {
        quote! {}
    };

    let decode_arms = messages.iter().map(|msg| {
        let opcode = msg.opcode as u16;
        let decode_body = build_message_decode_body(msg);

        quote! {
            #opcode => Ok({
                #decode_body
            })
        }
    });

    quote! {
        pub enum #incoming_enum_name #lifetime {
            #(#variants),*
        }

        impl #lifetime IncomingMessage<#message_type_marker> for #incoming_enum_name #lifetime {
            type Interface = #interface_name;
            fn try_decode(
                interface: &str,
                opcode: u16,
                message_type: denali_core::message::MessageType,
                data: &[u8],
            ) -> Result<Self, denali_core::message::DecodeMessageError> {
                if message_type != #message_type_enum {
                    todo!()
                }
                if interface != Self::Interface::INTERFACE {
                    return Err(denali_core::message::DecodeMessageError::UnknownInterface(interface.to_string()));
                }

                let mut reader = denali_core::wire::MessageDecoder::new(data);

                match opcode {
                    #(#decode_arms,)*
                    opcode => Err(denali_core::message::DecodeMessageError::UnknownOpcode(opcode))
                }
            }
        }
    }
}

fn build_outgoing_message_structs(
    ctx: &mut InterfaceCtx,
    interface_map: &BTreeMap<String, String>,
    interface: &Interface,
    message_type: MessageType,
) -> TokenStream {
    let interface_name = &ctx.interface_name;
    let suffix = if message_type == MessageType::Request {
        "Request"
    } else {
        "Event"
    };
    let prefix = &ctx.interface_str;

    let messages = interface.elements.iter().filter_map(|elem| match elem {
        InterfaceElement::Request(message) if message_type == MessageType::Request => Some(message),
        InterfaceElement::Event(message) if message_type == MessageType::Event => Some(message),
        _ => None,
    });

    let message_type_marker = if message_type == MessageType::Request {
        quote! { denali_core::message::Request }
    } else {
        quote! { denali_core::message::Event }
    };

    let structs = messages.map(|msg| {
        let name = build_ident(&format!("{prefix}_{}{suffix}", msg.name), Case::Pascal);
        let opcode = msg.opcode as u16;

        let (fields, uses_lifetime) = build_message_fields(ctx, interface_map, msg, true);

        let lifetime = if uses_lifetime {
            quote! { 'a }
        } else {
            quote! {}
        };

        let bound_generics = if msg.new_id_type == NewIdType::Generic {
            quote! { <#lifetime, I: Interface>  }
        } else {
            quote! { <#lifetime> }
        };
        let generics = if msg.new_id_type == NewIdType::Generic {
            quote! { <#lifetime, I>  }
        } else {
            quote! { <#lifetime> }
        };

        let response = match &msg.new_id_type {
            NewIdType::None => quote! { () },
            NewIdType::Generic => quote! {  ObjectId<I> },
            NewIdType::Typed { interface } => {
                let interface_path = interface_path(interface_map, interface);

                quote! {
                     ObjectId<#interface_path>
                }
            }
        };

        let encode_impl = build_message_encode_impl(msg, &name, &bound_generics, &generics);

        quote! {
            pub struct #name #bound_generics #fields

            impl #bound_generics OutgoingMessage<#message_type_marker> for #name #generics {
                type Interface = #interface_name;
                const OPCODE: u16 = #opcode;

                /// The type of response expected from this event/request.
                type Response = #response;

                fn sender(&self) -> &ObjectId<Self::Interface> {
                    &self.sender
                }
            }

            #encode_impl
        }
    });

    quote! {
        #(#structs)*
    }
}

fn build_interface(
    ctx: &InterfaceCtx,
    interface: &Interface,
    _interface_map: &BTreeMap<String, String>,
) -> TokenStream {
    let documentation = build_documentation(Some(&interface.description), None, None, None);
    let interface_str = interface
        .name
        .without_boundaries(&[Boundary::LOWER_DIGIT])
        .to_case(Case::Snake);
    let name = &ctx.interface_name;
    let version = interface.version;

    let event_enum_name = &ctx.event_enum_name;
    let event_lifetime = if ctx.event_enum_needs_lifetime {
        quote! { <'a> }
    } else {
        quote! {}
    };
    let request_enum_name = &ctx.request_enum_name;
    let request_lifetime = if ctx.request_enum_needs_lifetime {
        quote! { <'a> }
    } else {
        quote! {}
    };

    quote! {
        #documentation
        pub struct #name(());

        impl Interface for #name {
            const INTERFACE: &'static str = #interface_str;

            const MAX_VERSION: u32 = #version;

            type Event<'a> = #event_enum_name #event_lifetime;
            type Request<'a> = #request_enum_name #request_lifetime;
        }
    }
}

pub fn build_interface_module(
    interface: &Interface,
    interface_map: &BTreeMap<String, String>,
) -> TokenStream {
    let interface_module_name = build_ident(&interface.name, Case::Snake);
    let interface_type_name = build_ident(&interface.name, Case::Pascal);

    let event_enum_name = build_ident(&format!("{}Event", interface.name), Case::Pascal);
    let request_enum_name = build_ident(&format!("{}Request", interface.name), Case::Pascal);
    let mut ctx = InterfaceCtx {
        interface_name: interface_type_name,
        interface_str: interface.name.clone(),
        event_enum_name,
        request_enum_name,
        event_enum_needs_lifetime: false,
        request_enum_needs_lifetime: false,
    };

    let interface_desc = build_documentation(Some(&interface.description), None, None, None);
    let interface_version = interface.version;

    let enums = interface.elements.iter().filter_map(|element| {
        if let InterfaceElement::Enum(enum_) = element {
            Some(build_enum(enum_))
        } else {
            None
        }
    });

    let incoming_event_enum =
        build_incoming_message_enums(&mut ctx, interface_map, interface, MessageType::Event);
    let incoming_request_enum =
        build_incoming_message_enums(&mut ctx, interface_map, interface, MessageType::Request);

    let outgoing_event_structs =
        build_outgoing_message_structs(&mut ctx, interface_map, interface, MessageType::Event);
    let outgoing_request_structs =
        build_outgoing_message_structs(&mut ctx, interface_map, interface, MessageType::Request);

    let interface = build_interface(&ctx, interface, interface_map);

    quote! {
        #interface_desc
        pub mod #interface_module_name {
            use denali_core::prelude::*;

            pub const VERSION: u32 = #interface_version;

            #interface

            #incoming_event_enum
            #incoming_request_enum

            #outgoing_event_structs
            #outgoing_request_structs

            #(#enums)*
        }
    }
}
