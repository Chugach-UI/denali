use std::{collections::BTreeMap, os::fd};

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    build_ident,
    helpers::{build_documentation, expand_argument_type},
    protocol_parser::{Arg, Element, Interface, Request},
    wire::{build_enum, build_event, build_request},
};

fn build_request_method_body(
    request: &Request,
    new_id_arg: &Option<&Arg>,
    return_type: &TokenStream,
) -> TokenStream {
    let new_id_generic = matches!(
        new_id_arg,
        Some(Arg {
            interface: None,
            ..
        })
    );

    // Create the new ID if needed, statically or dynamically typed
    let version = if new_id_generic {
        quote! {
            version
        }
    } else {
        quote! {
            self.0.version()
        }
    };
    let new_id = if new_id_generic {
        quote! {
            let interface = <#return_type as denali_utils::Interface>::INTERFACE;
            let new_id = denali_utils::wire::serde::DynamicallyTypedNewId {
                interface: denali_utils::wire::serde::String::from(interface),
                version,
                id,
            };
        }
    } else {
        quote! {
            let new_id = id;
        }
    };

    // Only return the new object if there is a new_id argument
    let return_expr = if new_id_arg.is_some() {
        quote! {
            new_obj
        }
    } else {
        quote! {()}
    };

    let create_obj = if new_id_arg.is_some() {
        //TODO: AAAHAHAH
        quote! {
            let version = #version;
            let new_obj: #return_type = self.0.create_object(version).unwrap();
            let id = denali_utils::Object::id(&new_obj);

            #new_id
        }
    } else {
        quote! {}
    };

    // Build the request args type
    let request_struct = build_ident(&format!("{}Request", request.name), Case::Pascal);

    // Arguments that can be directly passed into the request unmodified.
    // New IDs and FDs need special handling, as FDs are encoded differently and new IDs aren't passed by the user.
    let passthrough_args = request
        .args
        .iter()
        .filter(|arg| arg.type_ != "new_id" && arg.type_ != "fd")
        .map(|arg| {
            let name = build_ident(&arg.name, Case::Snake);
            quote! { #name }
        });
    let fd_args = request
        .args
        .iter()
        .filter(|arg| arg.type_ == "fd")
        .map(|arg| {
            let name = build_ident(&arg.name, Case::Snake);
            quote! { #name: () }
        });
    let new_id_arg = if let Some(new_id_arg) = new_id_arg {
        let name = build_ident(&new_id_arg.name, Case::Snake);
        quote! { #name: new_id }
    } else {
        quote! {}
    };

    let create_request = quote! {
        let request = #request_struct {
            #(#passthrough_args,)*
            #(#fd_args,)*
            #new_id_arg
        };
    };

    quote! {
        #create_obj

        #create_request

        Ok(#return_expr)
    }
}

pub fn build_request_method(
    request: &Request,
    interface_map: &BTreeMap<String, String>,
) -> TokenStream {
    let name = request.name.to_case(Case::Snake);
    let name = name.trim_start_matches("get_");
    let try_name = build_ident(&format!("try_{name}"), Case::Snake);
    let name = build_ident(name, Case::Snake);

    let doc = build_documentation(&request.description, &None, &None, &None);

    let self_ = if request.type_.as_ref().is_some_and(|t| t == "destructor") {
        quote! { self }
    } else {
        quote! { &self }
    };

    let mut arg_names = request
        .args
        .iter()
        .filter(|arg| arg.type_ != "new_id")
        .map(|arg| build_ident(&arg.name, Case::Snake))
        .collect::<Vec<_>>();

    let mut args = request
        .args
        .iter()
        .filter(|arg| arg.type_ != "new_id")
        .map(|arg| {
            let name = build_ident(&arg.name, Case::Snake);
            let arg_type = expand_argument_type(arg, interface_map, None);
            quote! { #name: #arg_type }
        })
        .collect::<Vec<_>>();

    let new_id_arg = request.args.iter().find(|arg| arg.type_ == "new_id");

    let (generic, ret) = match new_id_arg {
        Some(Arg {
            interface: Some(interface),
            ..
        }) => {
            let protocol = interface_map
                .get(interface)
                .expect("Interface not found in interface map");
            let protocol = build_ident(protocol, Case::Snake);

            let interface_mod = build_ident(interface, Case::Snake);
            let interface_type = build_ident(interface, Case::Pascal);

            let type_path = quote! { super::super::#protocol::#interface_mod::#interface_type };

            (quote! {}, type_path)
        }
        Some(Arg { .. }) => {
            args.push(quote! { version: u32 });
            arg_names.push(build_ident("version", Case::Snake));

            let generic = quote! { <T: denali_utils::Interface> };

            (generic, quote! { T })
        }
        None => (quote! {}, quote! {()}),
    };

    let body = build_request_method_body(request, &new_id_arg, &ret);

    quote! {
        #doc
        /// # Errors
        ///
        /// This method will return an error if the request fails to be sent/serialized or if the response cannot be deserialized.
        pub fn #try_name #generic (#self_, #(#args),*) -> Result<#ret, denali_utils::wire::serde::SerdeError> {
            #body
        }
        #doc
        pub fn #name #generic (#self_, #(#args),*) -> #ret {
            match self.#try_name(#(#arg_names),*) {
                Ok(ret) => ret,
                Err(err) => panic!("Failed to send request: {}", err),
            }
        }
    }
}

//TODO: DO SERVER SIDE CODEGEN AS WELL
pub fn build_interface(
    interface: &Interface,
    interface_map: &BTreeMap<String, String>,
) -> TokenStream {
    let documentation = build_documentation(&interface.description, &None, &None, &None);
    let interface_str = interface.name.to_case(Case::Snake);
    let name = build_ident(&interface.name, Case::Pascal);
    let version = interface.version;

    let methods = interface.elements.iter().filter_map(|element| {
        if let Element::Request(request) = element {
            Some(build_request_method(request, interface_map))
        } else {
            None
        }
    });

    quote! {
        #documentation
        pub struct #name(denali_utils::proxy::Proxy);

        impl #name {
            #(#methods)*
        }

        impl From<denali_utils::proxy::Proxy> for #name {
            fn from(proxy: denali_utils::proxy::Proxy) -> Self {
                Self(proxy)
            }
        }

        impl denali_utils::Object for #name {
            fn id(&self) -> u32 {
                self.0.id()
            }
        }
        impl denali_utils::Interface for #name {
            const INTERFACE: &'static str = #interface_str;

            const MAX_VERSION: u32 = #version;
        }
    }
}

pub fn build_interface_module(
    interface: &Interface,
    interface_map: &BTreeMap<String, String>,
) -> TokenStream {
    let interface_name = build_ident(&interface.name, Case::Snake);
    let interface_desc = build_documentation(&interface.description, &None, &None, &None);
    let interface_version = interface.version;

    let events = interface.elements.iter().map(|element| match element {
        Element::Event(event) => Some(build_event(event, interface_map)),
        Element::Request(request) => Some(build_request(request, interface_map)),
        Element::Enum(enum_) => Some(build_enum(enum_)),
    });

    let interface = build_interface(interface, interface_map);

    quote! {
        #interface_desc
        pub mod #interface_name {
            pub const VERSION: u32 = #interface_version;

            #interface

            #(#events)*
        }
    }
}
