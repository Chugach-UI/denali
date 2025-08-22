use std::collections::BTreeMap;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    helper::{arg_type_to_rust_type, description_to_docstring},
    protocol_parser::{Enum, Event},
};

pub fn build_enum(enum_: &Enum) -> TokenStream {
    #[derive(PartialEq, Eq)]
    enum EnumInnerType {
        U32,
        I32,
    }

    let bitfield = enum_.bitfield.unwrap_or(false);
    let name = format_ident!("{}", enum_.name.to_case(Case::Pascal));
    let description = description_to_docstring(&enum_.description);

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
            let pascal = entry.name.to_case(Case::Pascal);

            let name = if let Some(c) = pascal.chars().next()
                && c.is_ascii_digit()
            {
                format!("_{}", pascal)
            } else {
                pascal
            };

            format_ident!("{}", name)
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
            let desc = description_to_docstring(&entry.description);

            quote! {
                #desc
                #name = #value,
            }
        });

    //TODO: BITFIELD BITFLAGS GEN 
    quote! {
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
    }
}

pub fn build_event(event: &Event, interface_map: &BTreeMap<String, String>) -> TokenStream {
    let name = format_ident!("{}Event", event.name.to_case(Case::Pascal));
    let description = description_to_docstring(&event.description);

    let arg_names = event
        .args
        .iter()
        .map(|arg| {
            let arg_name = arg.name.to_case(Case::Snake);
            format_ident!("{}", arg_name)
        })
        .collect::<Vec<_>>();

    let struct_members = event
        .args
        .iter()
        .map(|arg| {
            let arg_name = format_ident!("{}", arg.name.to_case(Case::Snake));
            let arg_type = arg
                .enum_
                .as_ref()
                .map(|enum_| {
                    let enum_parts = enum_.split('.').collect::<Vec<_>>();
                    let path = if enum_parts.len() == 1 {
                        let ident = format_ident!("{}", enum_parts[0].to_case(Case::Pascal));
                        quote! { #ident }
                    } else if enum_parts.len() == 2 {
                        let protocol = interface_map.get(enum_parts[0]).unwrap_or_else(|| {
                            panic!("Protocol '{}' not found in interface map", enum_parts[0])
                        });
                        let protocol = format_ident!("{}", protocol.to_case(Case::Snake));

                        let interface = format_ident!("{}", enum_parts[0].to_case(Case::Snake));
                        let ident = format_ident!("{}", enum_parts[1].to_case(Case::Pascal));
                        quote! { super::super::#protocol::#interface::#ident }
                    } else {
                        panic!("Invalid enum path: {}", enum_);
                    };

                    quote! {#path}
                })
                .unwrap_or_else(|| arg_type_to_rust_type(&arg.type_, Some("'a")));
            quote! {
                #arg_name: #arg_type,
            }
        })
        .collect::<Vec<_>>();

    let lifetime = event
        .args
        .iter()
        .find(|arg| arg.type_ == "string" || arg.type_ == "array")
        .map(|_| quote! { 'a })
        .into_iter()
        .collect::<Vec<_>>();

    let args_with_size = event
        .args
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
        #description
        pub struct #name #(<#lifetime>)* {
            #(pub #struct_members)*
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
