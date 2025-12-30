use std::fs::File;

mod serde;

pub fn parse_protocol(protocol: File) -> Result<Protocol, quick_xml::DeError> {
    let serde_protocol = serde::parse_protocol(protocol)?;

    Ok(Protocol {
        name: serde_protocol.name,
        description: transform_description(serde_protocol.description),
        interfaces: serde_protocol
            .interfaces
            .into_iter()
            .map(transform_interface)
            .collect(),
    })
}

fn transform_description(desc: Option<serde::Description>) -> Description {
    desc.map(|desc| Description {
        summary: desc.summary,
        content: desc.content.unwrap_or_default(),
    })
    .unwrap_or_default()
}

fn transform_interface(iface: serde::Interface) -> Interface {
    let mut request_opcode: i32 = -1;
    let mut event_opcode: i32 = -1;

    let elements = iface
        .elements
        .into_iter()
        .map(|element| match element {
            serde::Element::Request(request) => {
                request_opcode += 1;
                InterfaceElement::Request(transform_message(request, request_opcode as _))
            }
            serde::Element::Event(event) => {
                event_opcode += 1;
                InterfaceElement::Event(transform_message(event, event_opcode as _))
            }
            serde::Element::Enum(enum_) => InterfaceElement::Enum(transform_enum(enum_)),
        })
        .collect();

    Interface {
        name: iface.name,
        version: iface.version,
        description: transform_description(iface.description),
        elements,
    }
}

fn transform_message(message: serde::Message, opcode: u32) -> Message {
    let message_type = match message.type_ {
        None => MessageKind::Default,
        Some(kind) if kind == "destructor" => MessageKind::Destructor,
        _ => unreachable!(),
    };

    let new_id_arg = message.args.iter().find(|arg| arg.type_ == "new_id");
    let new_id_type = match new_id_arg {
        Some(arg) if arg.interface.is_some() => NewIdType::Typed {
            interface: arg.interface.clone().unwrap(),
        },
        Some(_) => NewIdType::Generic,
        None => NewIdType::None,
    };

    Message {
        name: message.name,
        opcode,
        message_type,
        deprecated_since: message.deprecated_since,
        description: transform_description(message.description),
        args: message
            .args
            .clone()
            .into_iter()
            .filter(|arg| arg.type_ != "fd")
            .map(transform_arg)
            .collect(),
        fd_args: message
            .args
            .into_iter()
            .filter(|arg| arg.type_ == "fd")
            .map(|arg| FdArg {
                name: arg.name,
                summary: arg.summary.unwrap_or_default(),
                description: transform_description(arg.description),
            })
            .collect(),
        new_id_type,
    }
}

fn transform_arg(arg: serde::Arg) -> Arg {
    let arg_type = match arg.type_.as_str() {
        "object" => ArgType::ObjectId {
            interface: arg.interface,
            nullable: arg.allow_null.unwrap_or(false),
        },

        "new_id" if arg.interface.is_none() => ArgType::GenericNewId,
        "new_id" => ArgType::NewId {
            interface: arg.interface.unwrap(),
        },

        "uint" if arg.enum_.is_some() => ArgType::Enum {
            enum_name: arg.enum_.unwrap(),
        },
        "uint" => ArgType::Uint,

        "int" => ArgType::Int,
        "fixed" => ArgType::Fixed,

        "string" => ArgType::String,
        "array" => ArgType::Array,

        "fd" => panic!("internal error. an fd arg was not filtered out properly"),

        _ => panic!("Unknown argument type '{}'", arg.type_),
    };

    Arg {
        name: arg.name,
        arg_type,
        summary: arg.summary.unwrap_or_default(),
        description: transform_description(arg.description),
    }
}

fn transform_enum(enum_: serde::Enum) -> Enum {
    Enum {
        name: enum_.name,
        bitfield: enum_.bitfield.unwrap_or(false),
        variants: enum_
            .entries
            .into_iter()
            .map(transform_enum_variant)
            .collect(),
    }
}

fn transform_enum_variant(variant: serde::Entry) -> EnumVariant {
    let value = if variant.value.contains("0x") {
        i64::from(u32::from_str_radix(variant.value.trim_start_matches("0x"), 16).unwrap())
    } else {
        variant.value.parse::<i64>().unwrap_or_else(|_| {
            panic!(
                "Failed to parse value '{}' for enum entry '{}'",
                variant.value, variant.name
            )
        })
    };

    EnumVariant {
        name: variant.name,
        value,
        summary: variant.summary.unwrap_or_default(),
        deprecated_since: variant.deprecated_since,
        description: transform_description(variant.description),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Protocol {
    pub name: String,
    pub description: Description,
    pub interfaces: Vec<Interface>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    pub name: String,
    pub version: u32,
    pub description: Description,
    pub elements: Vec<InterfaceElement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterfaceElement {
    Request(Message),
    Event(Message),
    Enum(Enum),
}
impl InterfaceElement {
    const fn is_request(&self) -> bool {
        matches!(self, InterfaceElement::Request(_))
    }
    const fn is_event(&self) -> bool {
        matches!(self, InterfaceElement::Event(_))
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageKind {
    Destructor,
    #[default]
    Default,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub name: String,
    pub opcode: u32,
    pub message_type: MessageKind,
    pub deprecated_since: Option<String>,
    pub description: Description,
    pub args: Vec<Arg>,
    pub new_id_type: NewIdType,
    pub fd_args: Vec<FdArg>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgType {
    GenericNewId,
    NewId {
        interface: String,
    },
    ObjectId {
        interface: Option<String>,
        nullable: bool,
    },
    Enum {
        enum_name: String,
    },
    Uint,
    Int,
    Fixed,
    Array,
    // Fd,
    String,
}
impl ArgType {
    pub const fn is_nullable(&self) -> bool {
        matches!(self, ArgType::ObjectId { nullable: true, .. })
    }
    pub const fn is_new_id(&self) -> bool {
        matches!(self, ArgType::NewId { .. } | ArgType::GenericNewId)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewIdType {
    None,
    Generic,
    Typed { interface: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arg {
    pub name: String,
    pub arg_type: ArgType,
    pub summary: String,
    pub description: Description,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FdArg {
    pub name: String,
    pub summary: String,
    pub description: Description,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnumInnerType {
    U32,
    I32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enum {
    pub name: String,
    pub bitfield: bool,
    pub variants: Vec<EnumVariant>,
}
impl Enum {
    pub const fn inner_type(&self) -> EnumInnerType {
        if self.bitfield {
            EnumInnerType::U32
        } else {
            EnumInnerType::I32
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariant {
    pub name: String,
    pub value: i64, // i64 to fit cases of u32 max and negatives
    pub summary: String,
    pub deprecated_since: Option<String>,
    pub description: Description,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Description {
    pub summary: String,
    pub content: String,
}
