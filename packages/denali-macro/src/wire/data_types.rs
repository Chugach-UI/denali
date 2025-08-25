use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{build_ident, helpers::build_documentation, protocol_parser::Enum};

pub fn build_enum(enum_: &Enum) -> TokenStream {
    #[derive(PartialEq, Eq)]
    enum EnumInnerType {
        U32,
        I32,
    }

    let bitfield = enum_.bitfield.unwrap_or(false);
    let name = format_ident!("{}", enum_.name.to_case(Case::Pascal));
    let description = build_documentation(&enum_.description, &None, &enum_.since, &None);

    let inner_type = if bitfield {
        EnumInnerType::U32
    } else {
        EnumInnerType::I32
    };

    // TODO: HANDLE THE TYPE CORRECTLY
    let type_stream = if inner_type == EnumInnerType::U32 {
        quote! { u32 }
    } else {
        quote! { i32 }
    };

    let variant_names = enum_
        .entries
        .iter()
        .map(|entry| {
            let proper_case = if bitfield {
                Case::UpperSnake
            } else {
                Case::Pascal
            };

            build_ident(&entry.name, proper_case)
        })
        .collect::<Vec<_>>();
    let variant_values = enum_
        .entries
        .iter()
        .map(|entry| {
            let value = if entry.value.contains("0x") {
                u32::from_str_radix(entry.value.trim_start_matches("0x"), 16).unwrap()
            } else {
                entry.value.parse().unwrap_or_else(|_| {
                    panic!(
                        "Failed to parse value '{}' for enum entry '{}'",
                        entry.value, entry.name
                    )
                })
            };

            match inner_type {
                EnumInnerType::U32 => quote! { #value },
                EnumInnerType::I32 => {
                    let value = value as i32;
                    quote! { #value }
                }
            }
        })
        .collect::<Vec<_>>();

    let variants = enum_
        .entries
        .iter()
        .zip(variant_names.iter().zip(variant_values.iter()))
        .map(|(entry, (name, value))| {
            let desc = build_documentation(
                &entry.description,
                &entry.summary,
                &entry.since,
                &entry.deprecated_since,
            );

            if bitfield {
                quote! {
                    #desc
                    const #name = #value;
                }
            } else {
                quote! {
                    #desc
                    #name = #value,
                }
            }
        });

    match bitfield {
        // Non-bitfield enum case
        // A regular Rust enum is generated
        false => quote! {
            #[repr(#type_stream)]
            #description
            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            pub enum #name {
                #(#variants)*
            }
            impl denali_utils::wire::serde::MessageSize for #name {}
            impl denali_utils::wire::serde::CompileTimeMessageSize for #name {}
            impl denali_utils::wire::serde::Decode for #name {
                fn decode(data: &[u8]) -> Result<Self, denali_utils::wire::serde::SerdeError> {
                    let mut traverser = denali_utils::wire::MessageDecoder::new(data);
                    let value = traverser.read::<#type_stream>()?;
                    Ok(match value {
                        #(#variant_values => #name::#variant_names,)*
                        _ => return Err(denali_utils::wire::serde::SerdeError::InvalidEnumValue),
                    })
                }
            }
            impl denali_utils::wire::serde::Encode for #name {
                fn encode(&self, data: &mut [u8]) -> Result<usize, denali_utils::wire::serde::SerdeError> {
                    let mut traverser = denali_utils::wire::MessageEncoder::new(data);
                    match self {
                        #(val @ #name::#variant_names => traverser.write(&(*val as #type_stream))?,)*
                    }
                    Ok(traverser.position() as usize)
                }
            }
        },
        // Bitfield enum case
        // The bitflags crate is used to generate a bitflags type
        true => quote! {
            denali_utils::__bitflags::bitflags! {
                #description
                pub struct #name: #type_stream {
                    #(#variants)*
                }
            }
            impl denali_utils::wire::serde::MessageSize for #name {}
            impl denali_utils::wire::serde::CompileTimeMessageSize for #name {}
            impl denali_utils::wire::serde::Decode for #name {
                fn decode(data: &[u8]) -> Result<Self, denali_utils::wire::serde::SerdeError> {
                    let mut traverser = denali_utils::wire::MessageDecoder::new(data);
                    let value = traverser.read::<#type_stream>()?;
                    Self::from_bits(value).ok_or(denali_utils::wire::serde::SerdeError::InvalidEnumValue)
                }
            }
            impl denali_utils::wire::serde::Encode for #name {
                fn encode(&self, data: &mut [u8]) -> Result<usize, denali_utils::wire::serde::SerdeError> {
                    let mut traverser = denali_utils::wire::MessageEncoder::new(data);
                    traverser.write(&self.bits())?;
                    Ok(traverser.position() as usize)
                }
            }
        },
    }
}
