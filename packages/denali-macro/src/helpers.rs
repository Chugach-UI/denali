use std::collections::BTreeMap;

use convert_case::{Boundary, Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::quote;

use crate::protocol_parser::{Arg, ArgType, Description};

pub fn arg_type_to_rust_type(type_: &ArgType, lifetime: Option<&str>) -> TokenStream {
    let lifetime = lifetime
        .map(|l| syn::Lifetime::new(l, Span::call_site()))
        .into_iter();
    match type_ {
        ArgType::Uint
        | ArgType::Enum { .. }
        | ArgType::ObjectId { .. }
        | ArgType::NewId { .. }
        | ArgType::GenericNewId => quote! { u32 },
        ArgType::Int => quote! { i32 },
        ArgType::Fixed => quote! { denali_core::wire::fixed::Fixed },
        ArgType::String => quote! { denali_core::wire::serde::String #(<#lifetime>)* },
        ArgType::Array => quote! { denali_core::wire::serde::Array #(<#lifetime>)* },
        ArgType::Fd => quote! { () },
    }
}

pub fn build_documentation(
    description: Option<&Description>,
    summary: Option<&String>,
    since: Option<&String>,
    deprecated_since: Option<&String>,
) -> TokenStream {
    let description = description
        .cloned()
        .or_else(|| {
            summary.map(|summary| Description {
                summary: summary.clone(),
                content: "".to_string(),
            })
        })
        .unwrap_or_default();
    let summary = description.summary.trim();
    let content = description
        .content
        .lines()
        .map(|line| line.trim().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let since = since
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

const ILLEGAL_IDENTS: [&str; 47] = [
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "fn", "for", "if",
    "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "static",
    "struct", "trait", "type", "unsafe", "use", "where", "while", "async", "await", "dyn",
    "abstract", "become", "box", "do", "final", "macro", "override", "priv", "typeof", "unsized",
    "virtual", "yield", "try", "gen",
];

pub fn build_ident(name: &str, case: Case<'_>) -> syn::Ident {
    let name = name
        .without_boundaries(&[Boundary::LOWER_DIGIT])
        .to_case(case);

    let name = if name.chars().next().is_some_and(|c| c.is_ascii_digit())
        || ILLEGAL_IDENTS.contains(&name.as_str())
    {
        format!("_{name}")
    } else {
        name
    };

    syn::Ident::new(&name, Span::call_site())
}

pub fn expand_argument_type(
    arg: &Arg,
    interface_map: &BTreeMap<String, String>,
    lifetime: Option<&str>,
) -> TokenStream {
    match arg {
        Arg {
            arg_type: ArgType::Enum { enum_name: enum_ },
            ..
        } => {
            let enum_parts = enum_.split('.').collect::<Vec<_>>();
            let path = if enum_parts.len() == 1 {
                let ident = build_ident(enum_parts[0], Case::Pascal);
                quote! { #ident }
            } else if enum_parts.len() == 2 {
                let protocol = interface_map.get(enum_parts[0]).unwrap_or_else(|| {
                    panic!("Protocol '{}' not found in interface map", enum_parts[0])
                });

                let protocol = build_ident(protocol, Case::Snake);
                let interface = build_ident(enum_parts[0], Case::Snake);

                let ident = build_ident(enum_parts[1], Case::Pascal);

                quote! { super::super::#protocol::#interface::#ident }
            } else {
                panic!("Invalid enum path: {enum_}");
            };

            quote! {#path}
        }
        Arg {
            arg_type: ArgType::NewId { .. },
            ..
        } => quote! {
            denali_core::wire::serde::NewId
        },
        Arg {
            arg_type: ArgType::GenericNewId,
            ..
        } => {
            let lifetime = match lifetime {
                Some(l) => {
                    let lifetime = syn::Lifetime::new(l, Span::call_site());
                    quote! { <#lifetime> }
                }
                None => quote! {},
            };
            quote! {
                denali_core::wire::serde::DynamicallyTypedNewId #lifetime
            }
        }
        arg => arg_type_to_rust_type(&arg.arg_type, lifetime),
    }
}

pub fn is_size_known_at_compile_time(args: &[&Arg]) -> bool {
    args.iter().any(|arg| {
        arg.arg_type == ArgType::String
            || arg.arg_type == ArgType::Array
            || (arg.arg_type == ArgType::GenericNewId)
    })
}
