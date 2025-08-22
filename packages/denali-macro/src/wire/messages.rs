use std::collections::BTreeMap;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{helpers::{arg_type_to_rust_type, description_to_docstring}, protocol_parser::Event};


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
