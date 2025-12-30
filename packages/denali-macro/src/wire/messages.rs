use std::collections::BTreeMap;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    MessageType, build_ident,
    helpers::{
        arg_type_to_rust_type, build_documentation, expand_argument_type,
        is_size_known_at_compile_time,
    },
    protocol_parser::{Arg, ArgType, Description, Interface, Message},
};

fn arg_to_serde_type(arg: &Arg) -> TokenStream {
    let arg_name = build_ident(&arg.name, Case::Snake);
    let arg_access = quote! {
        self.#arg_name
    };

    match arg.arg_type {
        ArgType::GenericNewId => quote! {
            denali_core::wire::serde::DynamicallyTypedNewId {
                interface: denali_core::wire::serde::String::new(I::INTERFACE),
                version: I::MAX_VERSION,
                id: 0
            }
        },
        ArgType::NewId { .. } => quote! {
            0u32
        },
        ArgType::ObjectId { nullable: true, .. } => quote! {
            #arg_access.map(ObjectId::get).unwrap_or_default()
        },
        ArgType::ObjectId {
            nullable: false, ..
        } => quote! {
            #arg_access.get()
        },

        ArgType::Enum { .. }
        | ArgType::Uint
        | ArgType::Int
        | ArgType::Fixed
        | ArgType::Array
        | ArgType::String => quote! { #arg_access },
    }
}

pub fn build_message_encode_impl(
    message: &Message,
    message_ident: syn::Ident,
    bound_generics: &TokenStream,
    generics: &TokenStream,
) -> TokenStream {
    let size_conversions = message
        .args
        .iter()
        .map(arg_to_serde_type)
        .collect::<Vec<_>>();

    //TODO: remove unwraps on client ID exhaustion (very low priority)
    let encode_conversions = message
        .args
        .iter()
        .map(|arg| match arg.arg_type.clone() {
            ArgType::GenericNewId => {
                quote! {
                    unsafe {
                        denali_core::wire::serde::DynamicallyTypedNewId {
                            interface: denali_core::wire::serde::String::new(I::INTERFACE),
                            version: I::MAX_VERSION,
                            id: id_factory.peek_next_id().unwrap()
                        }
                    }
                }
            }
            ArgType::NewId { .. } => {
                quote! {
                    unsafe { id_factory.peek_next_id().unwrap() }
                }
            }
            _ => arg_to_serde_type(arg),
        })
        .collect::<Vec<_>>();

    quote! {
        impl #bound_generics denali_core::wire::serde::MessageSize for #message_ident #generics {
            fn size(&self) -> usize {
                let mut size = 0;

                #(
                    size += (#size_conversions).size();
                )*

                size
            }
        }

        impl #bound_generics denali_core::message::EncodeWithNewId for #message_ident #generics {
            fn encode(&self, data: &mut [u8], mut id_factory: denali_core::id::IdFactory<'_>) -> Result<usize, denali_core::wire::serde::SerdeError> {
                let mut encoder = denali_core::wire::MessageEncoder::new(data);

                #(
                    encoder.write(&#encode_conversions)?;
                )*

                Ok(encoder.position() as usize)
            }
        }
    }
}

// #[allow(clippy::too_many_lines)]
// fn build_message(
//     message: &Message,
//     message_type: MessageType,
//     interface: &Interface,
//     interface_map: &BTreeMap<String, String>,
// ) -> TokenStream {
//     let is_request = message_type == MessageType::Request;

//     let suffix = if is_request { "Request" } else { "Event" };

//     let mut opcode: u16 = 0;
//     for elem in &interface.elements {
//         match elem {
//             crate::protocol_parser::InterfaceElement::Request(req) if is_request => {
//                 if req.name == message.name {
//                     break;
//                 }
//                 opcode += 1;
//             }
//             crate::protocol_parser::InterfaceElement::Event(evt) if !is_request => {
//                 if evt.name == message.name {
//                     break;
//                 }
//                 opcode += 1;
//             }
//             _ => {}
//         }
//     }
//     let opcode = quote! { pub const OPCODE: u16 = #opcode; };

//     let name = format_ident!("{}{suffix}", message.name.to_case(Case::Pascal));
//     let docs = build_documentation(
//         Some(&message.description),
//         None,
//         None,
//         message.deprecated_since.as_ref(),
//     );

//     let arg_names = message
//         .args
//         .iter()
//         .map(|arg| build_ident(&arg.name, Case::Snake))
//         .collect::<Vec<_>>();

//     let mut needs_lifetime = false;
//     let struct_members = message
//         .args
//         .iter()
//         .map(|arg| {
//             let arg_name = build_ident(&arg.name, Case::Snake);
//             let arg_docs =
//                 build_documentation(Some(&arg.description), Some(&arg.summary), None, None);
//             let (arg_type, uses_lifetime) = expand_argument_type(arg, interface_map, Some("'a"));

//             needs_lifetime |= uses_lifetime;

//             quote! {
//                 #arg_docs
//                 pub #arg_name: #arg_type,
//             }
//         })
//         .collect::<Vec<_>>();

//     let lifetime = if needs_lifetime {
//         quote! { <'a> }
//     } else {
//         quote! {}
//     };

//     let args_with_size = message
//         .args
//         .iter()
//         .filter(|arg| arg.arg_type != ArgType::Fd)
//         .collect::<Vec<_>>();

//     let compile_time_size = if is_size_known_at_compile_time(&args_with_size) {
//         quote! {}
//     } else {
//         let size = if args_with_size.is_empty() {
//             quote! { 0 }
//         } else {
//             let arg_types_with_size = args_with_size
//                 .iter()
//                 .map(|arg| arg_type_to_rust_type(&arg.arg_type, None).0)
//                 .collect::<Vec<_>>();

//             quote! { #(#arg_types_with_size::SIZE)+* }
//         };
//         quote! {
//            impl #lifetime denali_core::wire::serde::CompileTimeMessageSize for #name #lifetime {
//                const SIZE: usize = #size;
//            }
//         }
//     };

//     quote! {
//         #docs
//         #[derive(Debug, Clone, PartialEq, Eq)]
//         pub struct #name #lifetime {
//             #(#struct_members)*
//         }
//         impl #lifetime #name #lifetime {
//             #opcode
//         }
//         impl #lifetime denali_core::wire::serde::MessageSize for #name #lifetime {
//             fn size(&self) -> usize {
//                 let mut size = 0;
//                 #(
//                     size += self.#arg_names.size();
//                 )*
//                 size
//             }
//         }
//         #compile_time_size
//         impl #lifetime denali_core::wire::serde::Decode for #name #lifetime {
//             fn decode(data: &[u8]) -> Result<Self, denali_core::wire::serde::SerdeError> {
//                 let mut traverser = denali_core::wire::MessageDecoder::new(data);

//                 #(
//                     let #arg_names = traverser.read()?;
//                 )*

//                 Ok(Self {
//                     #(#arg_names),*
//                 })
//             }
//         }
//         impl #lifetime denali_core::wire::serde::Encode for #name #lifetime {
//             fn encode(&self, data: &mut [u8]) -> Result<usize, denali_core::wire::serde::SerdeError> {
//                 let mut traverser = denali_core::wire::MessageEncoder::new(data);

//                 #(
//                     traverser.write(&self.#arg_names)?;
//                 )*

//                 Ok(traverser.position() as usize)
//             }
//         }
//     }
// }
