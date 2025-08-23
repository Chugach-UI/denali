use convert_case::{Boundary, Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::quote;

use crate::protocol_parser::Description;

pub fn arg_type_to_rust_type(type_: &str, lifetime: Option<&str>) -> TokenStream {
    let lifetime = lifetime
        .map(|l| syn::Lifetime::new(l, Span::call_site()))
        .into_iter();
    match type_ {
        "uint" => quote! { u32 },
        "int" => quote! { i32 },
        "fixed" => quote! { denali_utils::fixed::Fixed },
        "string" => quote! { denali_utils::wire::serde::String #(<#lifetime>)* },
        "object" => quote! { u32 },
        "new_id" => quote! { u32 },
        "array" => quote! { denali_utils::wire::serde::Array #(<#lifetime>)* },
        "fd" => quote! { () },
        _ => panic!("Unknown type: {}", type_),
    }
}

pub fn build_documentation(
    description: &Option<Description>,
    summary: &Option<String>,
    since: &Option<String>,
    deprecated_since: &Option<String>,
) -> TokenStream {
    let description = description
        .clone()
        .or_else(|| {
            summary.clone().map(|summary| Description {
                summary,
                content: None,
            })
        })
        .unwrap_or_default();
    let summary = description.summary.trim();
    let content = description
        .content
        .unwrap_or_default()
        .lines()
        .map(|line| line.trim().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let since = since
        .clone()
        .map(|since| format!("Since: v{since}"))
        .unwrap_or_default();

    let deprecation = if let Some(since) = deprecated_since {
        let since = format!("Deprecated since: v{since}");
        quote! {
            #[deprecated(note = #since)]
        }
    } else {
        quote! {}
    };

    let doc_content = format!("{summary}\n{content}\n{since}");

    quote! {
        #deprecation
        #[doc = #doc_content]
    }
}

pub fn build_ident(name: &str, case: Case) -> syn::Ident {
    syn::Ident::new(
        &name
            .without_boundaries(&[Boundary::LOWER_DIGIT])
            .to_case(case),
        Span::call_site(),
    )
}
