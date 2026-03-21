use std::collections::BTreeMap;

use convert_case::{Boundary, Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::quote;

use crate::{
    InterfaceMap,
    protocol_parser::{Arg, ArgType, Description},
};

pub fn arg_type_to_rust_type(
    type_: &ArgType,
    lifetime: Option<syn::Lifetime>,
) -> (TokenStream, bool) {
    let mut lifetime_used = false;
    let lifetime = lifetime.into_iter();
    let type_ = match type_ {
        ArgType::Uint
        | ArgType::Enum { .. }
        | ArgType::ObjectId { .. }
        | ArgType::NewId { .. }
        | ArgType::GenericNewId => quote! { u32 },
        ArgType::Int => quote! { i32 },
        ArgType::Fixed => quote! { denali_core::wire::fixed::Fixed },
        ArgType::String => {
            lifetime_used = true;
            quote! { denali_core::wire::serde::String #(<#lifetime>)* }
        }
        ArgType::Array => {
            lifetime_used = true;
            quote! { denali_core::wire::serde::Array #(<#lifetime>)* }
        } // ArgType::Fd => quote! { () },
    };

    (type_, lifetime_used)
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
                content: String::new(),
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
    let doc_content = doc_content.trim();

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

/// Returns arg type and a bool for if a lifetime was used
pub fn expand_argument_type(
    arg: &Arg,
    interface_map: &InterfaceMap,
    lifetime: Option<&str>,
    outgoing: bool,
) -> (TokenStream, bool) {
    let mut lifetime_used = false;
    let lifetime = lifetime.map(|sym| syn::Lifetime::new(sym, Span::call_site()));

    let expanded = match &arg.arg_type {
        crate::protocol_parser::ArgType::GenericNewId => {
            let lifetime = lifetime.into_iter();
            lifetime_used = true;
            quote! { denali_core::id::DynamicObjectId #(<#lifetime>)* }
        }
        crate::protocol_parser::ArgType::NewId { interface } => {
            let interface_path = interface_map.path_to_interface_type(interface);
            quote! { ObjectId<#interface_path> }
        }
        crate::protocol_parser::ArgType::ObjectId {
            interface,
            nullable,
        } => {
            let lifetime = lifetime.into_iter();

            let reference = if outgoing {
                lifetime_used = true;
                quote! { & #(#lifetime)* }
            } else {
                quote! {}
            };

            let id_type = if let Some(interface) = interface {
                let interface_path = interface_map.path_to_interface_type(interface);
                quote! { #reference ObjectId<#interface_path> }
            } else {
                quote! {
                    #reference AnyObjectId
                }
            };

            if *nullable {
                quote! {
                    Option<#id_type>
                }
            } else {
                quote! {
                    #id_type
                }
            }
        }
        crate::protocol_parser::ArgType::Enum { enum_name } => {
            interface_map.path_to_enum_type(enum_name)
        }

        arg_type => {
            let (type_, used) = arg_type_to_rust_type(arg_type, lifetime);
            lifetime_used = used;
            type_
        }
    };

    (expanded, lifetime_used)
}

pub fn is_size_known_at_compile_time(args: &[&Arg]) -> bool {
    args.iter().any(|arg| {
        arg.arg_type == ArgType::String
            || arg.arg_type == ArgType::Array
            || (arg.arg_type == ArgType::GenericNewId)
    })
}
