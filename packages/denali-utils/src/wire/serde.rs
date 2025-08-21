use std::{
    borrow::Cow,
    io::{Cursor, Write},
};

use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use paste::paste;
use thiserror::Error;

pub trait CompileTimeMessageSize: MessageSize {
    const SIZE: usize = size_of::<Self>();
}
pub trait MessageSize: Sized {
    fn size(&self) -> usize {
        size_of::<Self>()
    }
}

macro_rules! ensure_size {
    ($data:expr, $t:ident) => {
        if $data.len() < $t::SIZE {
            return Err(SerdeError::InvalidSize);
        }
    };
}
macro_rules! impl_serde {
    {
        #[$($attr:meta),*]
        $vis:vis struct $name:ident {
            $(pub $field:ident: $type:ty),* $(,)?
        }
    } => {
        #[$($attr),*]
        $vis struct $name {
            $(pub $field: $type),*
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

pub trait Decode: MessageSize {
    fn decode(data: &[u8]) -> Result<Self, SerdeError>;
}

pub trait Encode: MessageSize {
    fn encode(&self, data: &mut [u8]) -> Result<usize, SerdeError>;
}

impl_serde! {
    #[repr(C)]
    pub struct MessageHeader {
        pub object_id: u32,
        pub size: u16,
        pub opcode: u16,
    }
}
impl_serde!(u32, i32);

pub struct Array<'a> {
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

impl<'a> Decode for Array<'a> {
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
impl<'a> Encode for Array<'a> {
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

pub struct String<'a> {
    pub data: Cow<'a, str>,
}
impl<'a> String<'a> {
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

        let string_data = match std::str::from_utf8(&array_data[..size]) {
            Ok(s) => s,
            Err(_) => return Err(SerdeError::InvalidSize),
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

#[derive(Debug, Error)]
pub enum SerdeError {
    #[error("The data provided is not long enough to decode the expected type.")]
    InvalidSize,
    #[error("IO error occurred while decoding")]
    IoError(#[from] std::io::Error),
}
