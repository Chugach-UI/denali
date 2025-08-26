use std::{
    borrow::Cow,
    io::{Cursor, Write},
};

use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use paste::paste;
use thiserror::Error;

use crate::fixed::Fixed;

/// The size of a message/type in bytes when encoded for the Wayland wire protocol.
/// Types implementing this trait have a encoded size that is known at compile time.
///
/// For types that have an encoded size that can only be determined at runtime, implement only [`MessageSize`].
pub trait CompileTimeMessageSize: MessageSize {
    /// The size of this type when encoded for the Wayland wire protocol, in bytes.
    const SIZE: usize = size_of::<Self>();
}

/// The size of a message/type in bytes when encoded for the Wayland wire protocol.
/// Types implementing this trait have a encoded size that can be determined at runtime.
///
/// For types that have an encoded size that can be determined at compile time, implement both this and [`CompileTimeMessageSize`].
pub trait MessageSize: Sized {
    /// Returns the size of this type when encoded for the Wayland wire protocol, in bytes.
    fn size(&self) -> usize {
        size_of::<Self>()
    }
}

/// Ensures that the provided data slice is at least as large as the size of the type `$t`.
///
/// # Note
///
/// `$t` must implement [`CompileTimeMessageSize`].
#[macro_export]
macro_rules! ensure_size {
    ($data:expr, $t:ident) => {
        if $data.len() < $t::SIZE {
            return Err(SerdeError::InvalidSize);
        }
    };
}
pub use crate::ensure_size;

macro_rules! impl_serde {
    {
        $(#[$attr:meta])*
        $vis:vis struct $name:ident {
            $(
                $(#[$field_attr:meta])*
                pub $field:ident: $type:ty
            ),* $(,)?
        }
    } => {
        $(#[$attr])*
        $vis struct $name {
            $(
                $(#[$field_attr])*
                pub $field: $type
            ),*
        }
        impl MessageSize for $name {}
        impl CompileTimeMessageSize for $name {}
        impl Decode for $name {
            fn decode(data: &[u8]) -> Result<Self, SerdeError> {
                ensure_size!(data, Self);
                let mut data = Cursor::new(data);
                paste! {
                    Ok(Self {
                        $($field:  data.[<read_ $type>]::<LE>()?),*
                    })
                }
            }
        }
        impl Encode for $name {
            fn encode(&self, data: &mut [u8]) -> Result<usize, SerdeError> {
                ensure_size!(data, Self);
                let mut data = Cursor::new(data);
                paste! {
                    $(data.[<write_ $type>]::<LE>(self.$field)?);*
                }
                Ok(Self::SIZE)
            }
        }
    };
    (
        $(
            $type:ty
        ),*
    ) => {
        $(
            impl CompileTimeMessageSize for $type {
                const SIZE: usize = size_of::<$type>();
            }
            impl MessageSize for $type {
                fn size(&self) -> usize {
                    Self::SIZE
                }
            }
            impl Decode for $type {
                fn decode(data: &[u8]) -> Result<Self, SerdeError> {
                    ensure_size!(data, Self);
                    let mut data = Cursor::new(data);
                    paste! {
                        Ok(data.[<read_ $type>]::<LE>()? as _)
                    }
                }
            }
            impl Encode for $type {
                fn encode(&self, data: &mut [u8]) -> Result<usize, SerdeError> {
                    ensure_size!(data, Self);
                    let mut data = Cursor::new(data);
                    paste! {
                        data.[<write_ $type>]::<LE>(*self as _)?;
                    }
                    Ok(Self::SIZE)
                }
            }
        )*
    };
}

/// A type that can be decoded from the Wayland wire protocol.
pub trait Decode: MessageSize {
    /// Decodes an instance of this type from the provided byte slice.
    ///
    /// # Errors
    ///
    /// This function returns errors if:
    /// - The provided data slice is not large enough to contain the expected type.
    /// - An IO error occurs while reading from the data slice.
    /// - An invalid enum value is encountered while decoding an enum type.
    fn decode(data: &[u8]) -> Result<Self, SerdeError>;
}

/// A type that can be encoded to the Wayland wire protocol.
pub trait Encode: MessageSize {
    /// Encodes this instance into the provided byte slice.
    ///
    /// # Errors
    ///
    /// This function returns errors if:
    /// - The provided data slice is not large enough to contain the encoded type.
    /// - An IO error occurs while writing to the data slice.
    /// - An invalid enum value is encountered while encoding an enum type.
    fn encode(&self, data: &mut [u8]) -> Result<usize, SerdeError>;
}

impl_serde! {
    /// The header of a Wayland message.
    #[repr(C)]
    pub struct MessageHeader {
        /// The ID of the object the message is for.
        pub object_id: u32,
        /// The opcode of the request/event.
        pub opcode: u16,
        /// The size of the message in bytes, including the header.
        pub size: u16,
    }
}
impl_serde!(u32, i32);

impl MessageSize for () {}
impl CompileTimeMessageSize for () {}
impl Decode for () {
    fn decode(_data: &[u8]) -> Result<Self, SerdeError> {
        Ok(())
    }
}
impl Encode for () {
    fn encode(&self, _data: &mut [u8]) -> Result<usize, SerdeError> {
        Ok(0)
    }
}

impl MessageSize for Fixed {}
impl CompileTimeMessageSize for Fixed {}
impl Decode for Fixed {
    fn decode(data: &[u8]) -> Result<Self, SerdeError> {
        ensure_size!(data, Fixed);
        let mut cursor = Cursor::new(data);
        let value = cursor.read_i32::<LE>()?;
        Ok(Fixed(value))
    }
}
impl Encode for Fixed {
    fn encode(&self, data: &mut [u8]) -> Result<usize, SerdeError> {
        ensure_size!(data, Fixed);
        let mut cursor = Cursor::new(data);
        cursor.write_i32::<LE>(self.0)?;
        Ok(Fixed::SIZE)
    }
}

/// A unique object ID
pub type ObjectId = u32;

/// A statically typed new ID.
pub type NewId = ObjectId;
/// A dynamically typed new ID.
pub struct DynamicallyTypedNewId<'a> {
    /// The interface name of the new object.
    pub interface: String<'a>,
    /// The version of the new object.
    pub version: u32,
    /// The ID of the new object.
    pub id: ObjectId,
}
impl MessageSize for DynamicallyTypedNewId<'_> {
    fn size(&self) -> usize {
        self.interface.size() + u32::SIZE + ObjectId::SIZE
    }
}
impl Decode for DynamicallyTypedNewId<'_> {
    fn decode(data: &[u8]) -> Result<Self, SerdeError> {
        let mut traverser = super::MessageDecoder::new(data);

        let interface: String<'_> = traverser.read()?;
        let version = traverser.read()?;
        let id = traverser.read()?;
        Ok(DynamicallyTypedNewId {
            interface,
            version,
            id,
        })
    }
}
impl Encode for DynamicallyTypedNewId<'_> {
    fn encode(&self, data: &mut [u8]) -> Result<usize, SerdeError> {
        let mut traverser = super::MessageEncoder::new(data);

        traverser.write(&self.interface)?;
        traverser.write(&self.version)?;
        traverser.write(&self.id)?;
        Ok(self.size())
    }
}

/// A dynamically sized array of bytes.
pub struct Array<'a> {
    /// The raw byte data of the array.
    pub data: Cow<'a, [u8]>,
}
impl From<Vec<u8>> for Array<'_> {
    fn from(value: Vec<u8>) -> Self {
        Self { data: value.into() }
    }
}
impl<'a> From<&'a [u8]> for Array<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self { data: value.into() }
    }
}
impl<const N: usize> From<[u8; N]> for Array<'_> {
    fn from(value: [u8; N]) -> Self {
        Self {
            data: value.to_vec().into(),
        }
    }
}
impl<'a> From<Cow<'a, [u8]>> for Array<'a> {
    fn from(value: Cow<'a, [u8]>) -> Self {
        Self { data: value }
    }
}

impl MessageSize for Array<'_> {
    fn size(&self) -> usize {
        self.data.len() + 4 // 4 bytes for the size of the array
    }
}

impl Decode for Array<'_> {
    fn decode(data: &[u8]) -> Result<Self, SerdeError> {
        ensure_size!(data, u32);

        let mut cursor = Cursor::new(data);
        let size = cursor.read_u32::<LE>()? as usize;

        if data.len() < size + 4 {
            return Err(SerdeError::InvalidSize);
        }

        let array_data = &data[4..size + 4];

        Ok(Array {
            // TODO: REMOVE USAGE OF HEAP HERE!!!
            data: array_data.to_owned().into(),
        })
    }
}
impl Encode for Array<'_> {
    fn encode(&self, data: &mut [u8]) -> Result<usize, SerdeError> {
        let size = self.size();
        if data.len() < size {
            return Err(SerdeError::InvalidSize);
        }

        let mut cursor = Cursor::new(data);
        cursor.write_u32::<LE>(self.data.len() as u32)?;
        cursor.write_all(&self.data)?;

        Ok(size)
    }
}

/// A dynamically sized UTF-8 string.
pub struct String<'a> {
    /// The UTF-8 string data.
    pub data: Cow<'a, str>,
}
impl<'a> String<'a> {
    /// Creates a new `String` from the provided data.
    #[must_use]
    pub fn new(data: impl Into<Cow<'a, str>>) -> Self {
        Self { data: data.into() }
    }
}
impl From<std::string::String> for String<'_> {
    fn from(value: std::string::String) -> Self {
        Self { data: value.into() }
    }
}
impl<'a> From<&'a str> for String<'a> {
    fn from(value: &'a str) -> Self {
        Self { data: value.into() }
    }
}
impl<'a> From<Cow<'a, str>> for String<'a> {
    fn from(value: Cow<'a, str>) -> Self {
        Self { data: value }
    }
}

impl MessageSize for String<'_> {
    fn size(&self) -> usize {
        self.data.len() + 5 // 4 bytes for the size of the string + 1 for the null terminator
    }
}

impl Decode for String<'_> {
    fn decode(data: &[u8]) -> Result<Self, SerdeError> {
        ensure_size!(data, u32);

        let mut cursor = Cursor::new(data);
        let size = cursor.read_u32::<LE>()? as usize;

        if data.len() < size + 5 {
            return Err(SerdeError::InvalidSize);
        }

        let array_data = &data[4..size + 5];
        assert!(
            array_data.ends_with(&[0]),
            "String data must end with a null terminator"
        );

        let Ok(string_data) = std::str::from_utf8(&array_data[..size]) else {
            return Err(SerdeError::InvalidSize);
        };

        Ok(Self {
            //TODO: Remove heap usage!!!
            data: string_data.to_owned().into(),
        })
    }
}
impl Encode for String<'_> {
    fn encode(&self, data: &mut [u8]) -> Result<usize, SerdeError> {
        let size = self.size();
        if data.len() < size {
            return Err(SerdeError::InvalidSize);
        }

        let mut cursor = Cursor::new(data);
        cursor.write_u32::<LE>(self.data.len() as u32)?;
        cursor.write_all(self.data.as_bytes())?;
        cursor.write_u8(0)?; // null terminator

        Ok(size)
    }
}

/// Errors that can occur during serialization/deserialization of Wayland wire protocol messages.
#[derive(Debug, Error)]
pub enum SerdeError {
    /// The buffer provided is not long enough to encode/decode the expected type. 
    #[error("The data provided is not long enough to encode/decode the expected type.")]
    InvalidSize,
    /// An IO error occurred while encoding/decoding.
    #[error("IO error occurred while decoding")]
    IoError(#[from] std::io::Error),
    /// An invalid enum value was encountered while encoding/decoding.
    #[error("Invalid enum value")]
    InvalidEnumValue,
}
