use proc_macro2::TokenStream;
use quote::quote;

use crate::protocol_parser::Description;

pub fn arg_type_to_rust_type(type_: &str) -> TokenStream {
    match type_ {
        "uint" => quote! { u32 },
        "int" => quote! { i32 },
        "fixed" => quote! { denali_utils::fixed::Fixed },
        "string" => quote! { denali_utils::wire::serde::String },
        "object" => quote! { u32 },
        "new_id" => quote! { u32 },
        "array" => quote! { denali_utils::wire::serde::Array },
        "fd" => quote! { () },
        _ => panic!("Unknown type: {}", type_),
    }
}
pub fn description_to_docstring(description: &Option<Description>) -> TokenStream {
    let description = description.clone().unwrap_or_default();
    let description = format!(
        "{}\n{}",
        description.summary,
        description.content.unwrap_or_default()
    );

    quote! {
        #[doc = #description]
    }
}