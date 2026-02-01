use convert_case::Case;
use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    build_ident,
    protocol_parser::{Arg, ArgType, Message},
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
    message_ident: &syn::Ident,
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

fn read_into_arg_var(arg: &Arg) -> TokenStream {
    let arg_name = build_ident(&arg.name, Case::Snake);

    match &arg.arg_type {
        ArgType::GenericNewId => quote! {
            let raw: denali_core::wire::serde::DynamicallyTypedNewId = reader.read()?;
            let #arg_name = denali_core::id::DynamicObjectId::new(raw.id, raw.version, raw.interface.data);
        },
        ArgType::ObjectId {
            nullable,
            interface,
        } => {
            let id = if interface.is_some() {
                quote! { ObjectId::from_raw(raw) }
            } else {
                quote! { AnyObjectId::new(raw) }
            };

            if *nullable {
                quote! {
                    let raw = reader.read()?;
                    let #arg_name = if raw != 0 {
                        unsafe { Some(#id) }
                    } else {
                        None
                    };
                }
            } else {
                quote! {
                    let raw = reader.read()?;
                    let #arg_name = unsafe { #id };
                }
            }
        }
        ArgType::NewId { .. } => quote! {
            let raw = reader.read()?;
            let #arg_name = unsafe { ObjectId::from_raw(raw) };
        },

        ArgType::Enum { .. }
        | ArgType::Uint
        | ArgType::Int
        | ArgType::Fixed
        | ArgType::Array
        | ArgType::String => quote! { let #arg_name = reader.read()?; },
    }
}

pub fn build_message_decode_body(message: &Message) -> TokenStream {
    let definitions = message.args.iter().map(read_into_arg_var);
    let message_ident = build_ident(&message.name, Case::Pascal);

    let arg_names = message
        .args
        .iter()
        .map(|arg| build_ident(&arg.name, Case::Snake));

    quote! {
        #(#definitions)*

        Self::#message_ident {
            #(#arg_names),*
        }
    }
}
