use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    build_ident,
    helpers::build_documentation,
    protocol_parser::{Enum, EnumInnerType},
};

#[allow(clippy::too_many_lines)]
pub fn build_enum(enum_: &Enum) -> TokenStream {
    let name = format_ident!("{}", enum_.name.to_case(Case::Pascal));
    let description = build_documentation(None, None, None, None);

    let inner_type = enum_.inner_type();

    // TODO: HANDLE THE TYPE CORRECTLY
    let type_stream = if inner_type == EnumInnerType::U32 {
        quote! { u32 }
    } else {
        quote! { i32 }
    };

    let variant_names = enum_
        .variants
        .iter()
        .map(|entry| {
            let proper_case = if enum_.bitfield {
                Case::UpperSnake
            } else {
                Case::Pascal
            };

            build_ident(&entry.name, proper_case)
        })
        .collect::<Vec<_>>();
    let variant_values = enum_
        .variants
        .iter()
        .map(|entry| {
            let value = entry.value;

            match inner_type {
                EnumInnerType::U32 => {
                    let value = value as u32;
                    quote! { #value }
                }
                EnumInnerType::I32 => {
                    let value = value as i32;
                    quote! { #value }
                }
            }
        })
        .collect::<Vec<_>>();

    let variants = enum_
        .variants
        .iter()
        .zip(variant_names.iter().zip(variant_values.iter()))
        .map(|(entry, (name, value))| {
            let desc =
                build_documentation(Some(&entry.description), Some(&entry.summary), None, None);

            if enum_.bitfield {
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

    if enum_.bitfield {
        quote! {
            denali_core::__bitflags::bitflags! {
                #description
                #[derive(Clone, Copy, Debug, PartialEq, Eq)]
                pub struct #name: #type_stream {
                    #(#variants)*
                }
            }
            impl denali_core::wire::serde::MessageSize for #name {
                fn size(&self) -> usize {
                    size_of::<Self>()
                }
            }
            impl denali_core::wire::serde::CompileTimeMessageSize for #name {
                const SIZE: usize = size_of::<Self>();
            }
            impl<'de> denali_core::wire::serde::Decode<'de> for #name {
                fn decode(data: &'de [u8]) -> Result<Self, denali_core::wire::serde::SerdeError> {
                    let mut traverser = denali_core::wire::MessageDecoder::new(data);
                    let value = traverser.read::<#type_stream>()?;
                    Self::from_bits(value).ok_or(denali_core::wire::serde::SerdeError::InvalidEnumValue)
                }
            }
            impl denali_core::wire::serde::Encode for #name {
                fn encode(&self, data: &mut [u8]) -> Result<usize, denali_core::wire::serde::SerdeError> {
                    let mut traverser = denali_core::wire::MessageEncoder::new(data);
                    traverser.write(&self.bits())?;
                    Ok(traverser.position() as usize)
                }
            }
        }
    } else {
        quote! {
            #[repr(#type_stream)]
            #description
            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            pub enum #name {
                #(#variants)*
            }
            impl denali_core::wire::serde::MessageSize for #name {
                fn size(&self) -> usize {
                    size_of::<Self>()
                }
            }
            impl denali_core::wire::serde::CompileTimeMessageSize for #name {
                const SIZE: usize = size_of::<Self>();
            }
            impl<'de> denali_core::wire::serde::Decode<'de> for #name {
                fn decode(data: &'de [u8]) -> Result<Self, denali_core::wire::serde::SerdeError> {
                    let mut traverser = denali_core::wire::MessageDecoder::new(data);
                    let value = traverser.read::<#type_stream>()?;
                    Ok(match value {
                        #(#variant_values => #name::#variant_names,)*
                        _ => return Err(denali_core::wire::serde::SerdeError::InvalidEnumValue),
                    })
                }
            }
            impl denali_core::wire::serde::Encode for #name {
                fn encode(&self, data: &mut [u8]) -> Result<usize, denali_core::wire::serde::SerdeError> {
                    let mut traverser = denali_core::wire::MessageEncoder::new(data);
                    match self {
                        #(val @ #name::#variant_names => traverser.write(&(*val as #type_stream))?,)*
                    }
                    Ok(traverser.position() as usize)
                }
            }
        }
    }
}
