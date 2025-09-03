use convert_case::Case;
use proc_macro2::TokenStream;

use crate::{
    build_ident,
    helpers::{build_documentation, expand_argument_type},
    protocol_parser::{Arg, Request},
};
use std::collections::BTreeMap;

use convert_case::Casing;
use quote::quote;

fn build_request_method_body(
    request: &Request,
    new_id_arg: Option<&Arg>,
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
            let new_id = denali_core::wire::serde::DynamicallyTypedNewId {
                interface: denali_core::wire::serde::String::from(interface),
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

    let create_obj = if new_id_arg.is_some() && !new_id_generic {
        quote! {
            let version = #version;
            let new_obj: #return_type = self.0.create_object(version).unwrap();
            let id = denali_core::Object::id(&new_obj);

            #new_id
        }
    } else if new_id_generic {
        quote! {
            let version = #version;
            let new_obj = self.0.create_object_raw(interface, version).unwrap();
            let id = denali_core::Object::id(&new_obj);

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
            quote! { #name }
        })
        .collect::<Vec<_>>();
    let new_id_arg = if let Some(new_id_arg) = new_id_arg {
        let name = build_ident(&new_id_arg.name, Case::Snake);
        quote! { #name: new_id }
    } else {
        quote! {}
    };

    let create_request_requirements = quote! {
        use denali_core::{wire::serde::{MessageSize, CompileTimeMessageSize}, Object};

        let request = #request_struct {
            #(#passthrough_args,)*
            #(#fd_args: (),)*
            #new_id_arg
        };
        let object_id = self.id();
        let opcode = #request_struct::OPCODE;
        let size = request.size() + denali_core::wire::serde::MessageHeader::SIZE;

        let mut buffer = vec![0u8; size];
        let fds: Vec<std::os::fd::RawFd> = vec![#(#fd_args.into_raw_fd(),)*];

        denali_core::wire::encode_message(&request, object_id, opcode, &mut buffer)?;

        self.send_request(denali_core::proxy::RequestMessage { fds, buffer });
    };

    quote! {
        #create_obj

        #create_request_requirements

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

    let doc = build_documentation(request.description.as_ref(), None, None, None);

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
            let arg_type = match arg.type_.as_str() {
                "fd" => quote! { impl std::os::fd::IntoRawFd },
                _ => expand_argument_type(arg, interface_map, None),
            };
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

            let generic = quote! { <T: denali_core::Interface> };

            (generic, quote! { T })
        }
        None => (quote! {}, quote! {()}),
    };

    let has_raw_function = matches!(
        new_id_arg,
        Some(Arg {
            interface: None,
            ..
        })
    );

    let body = build_request_method_body(request, new_id_arg, &ret);

    let raw_name = build_ident(&format!("{name}_raw"), Case::Snake);

    let raw_function = if has_raw_function {
        quote! {
            #doc
            /// # Errors
            ///
            /// This method will return an error if the request fails to be sent/serialized or if the response cannot be deserialized.
            pub fn #raw_name (#self_, interface: &str, #(#args),*) -> Result<denali_core::proxy::Proxy, denali_core::wire::serde::SerdeError> {
                #body
            }
        }
    } else {
        quote! {}
    };

    let try_function_body = if has_raw_function {
        quote! {
            self.#raw_name(<#ret as denali_core::Interface>::INTERFACE, #(#arg_names),*).map(Into::into)
        }
    } else {
        quote! {
            #body
        }
    };

    quote! {
        #raw_function

        #doc
        /// # Errors
        ///
        /// This method will return an error if the request fails to be sent/serialized or if the response cannot be deserialized.
        pub fn #try_name #generic (#self_, #(#args),*) -> Result<#ret, denali_core::wire::serde::SerdeError> {
            #try_function_body
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
