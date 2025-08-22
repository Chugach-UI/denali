use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    helper::{arg_type_to_rust_type, description_to_docstring},
    protocol_parser::Event,
};

pub fn build_event(event: &Event) -> TokenStream {
    let name = format_ident!("{}", event.name.to_case(Case::Pascal));
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
            let arg_type = arg_type_to_rust_type(&arg.type_);
            quote! {
                #arg_name: #arg_type,
            }
        })
        .collect::<Vec<_>>();

    quote! {
        #description
        pub struct #name {
            #(pub #struct_members)*
        }
        impl #name {
            pub fn decode(data: &[u8]) -> Result<Self, denali_utils::wire::serde::SerdeError> {
                let mut traverser = denali_utils::wire::MessageDecoder::new(data);

                #(
                    let #arg_names = traverser.read()?;
                )*

                Ok(Self {
                    #(#arg_names),*
                })
            }
            pub fn encode(&self, data: &mut [u8]) -> Result<(), denali_utils::wire::serde::SerdeError> {
                let mut traverser = denali_utils::wire::MessageEncoder::new(data);

                #(
                    traverser.write(&self.#arg_names)?;
                )*

                Ok(())
            }
        }
    }
}
