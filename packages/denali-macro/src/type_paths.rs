use std::collections::{BTreeMap, HashMap};

use convert_case::Case;
use proc_macro2::TokenStream;
use quote::quote;

use crate::{Protocol, build_ident};

/// Map of interface to protocol name
pub type ExternalProtocolMap = BTreeMap<String, String>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolLocation {
    Local {
        protocol_name: String,
    },
    External {
        crate_name: String,
        protocol_name: String,
    },
}

pub struct InterfaceMap {
    inner: BTreeMap<String, ProtocolLocation>,
}
impl InterfaceMap {
    pub fn with_local_protocols(protocols: &[Protocol]) -> Self {
        let mut map = BTreeMap::new();

        for protocol in protocols {
            for interface in &protocol.interfaces {
                map.insert(
                    interface.name.clone(),
                    ProtocolLocation::Local {
                        protocol_name: protocol.name.clone(),
                    },
                );
            }
        }

        Self { inner: map }
    }

    pub fn add_external_protocols(&mut self, crate_name: &str, map: ExternalProtocolMap) {
        for (interface, protocol) in map {
            self.inner.insert(
                interface.clone(),
                ProtocolLocation::External {
                    crate_name: crate_name.to_string(),
                    protocol_name: protocol.clone(),
                },
            );
        }
    }

    pub fn interface_module_path(&self, interface: &str) -> TokenStream {
        let interface_ident = build_ident(interface, Case::Snake);

        let protocol = match &self.inner.get(interface) {
            Some(ProtocolLocation::Local { protocol_name }) => {
                let protocol = build_ident(protocol_name, Case::Snake);

                quote! {
                    super::super::#protocol
                }
            }
            Some(ProtocolLocation::External {
                crate_name,
                protocol_name,
            }) => {
                let crate_ = build_ident(crate_name, Case::Snake);
                let protocol = build_ident(protocol_name, Case::Snake);

                quote! { #crate_::#protocol }
            }
            None => panic!("Interface {interface} not found."),
        };

        quote! { #protocol::#interface_ident }
    }

    pub fn path_to_enum_type(&self, enum_name: &str) -> TokenStream {
        let enum_parts = enum_name.split('.').collect::<Vec<_>>();

        if enum_parts.len() == 1 {
            let ident = build_ident(enum_parts[0], Case::Pascal);
            return quote! { #ident };
        }

        if enum_parts.len() > 2 {
            panic!("Invalid enum path: {enum_name}");
        }

        let interface_mod_path = self.interface_module_path(enum_parts[0]);
        let ident = build_ident(enum_parts[1], Case::Pascal);

        quote! { #interface_mod_path::#ident }
    }

    pub fn path_to_interface_type(&self, interface_name: &str) -> TokenStream {
        let interface_mod_path = self.interface_module_path(interface_name);
        let ident = build_ident(interface_name, Case::Pascal);

        quote! { #interface_mod_path::#ident }
    }
}
