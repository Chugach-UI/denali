use std::io::Cursor;

use byteorder::{LE, ReadBytesExt, WriteBytesExt};
use paste::paste;
use thiserror::Error;

pub trait MessageSize: Sized {
    const SIZE: usize = size_of::<Self>();
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
            fn encode(&self, data: &mut [u8]) -> Result<(), SerdeError> {
                ensure_size!(data, Self);
                let mut data = Cursor::new(data);
                paste! {
                    $(data.[<write_ $type>]::<LE>(self.$field)?);*
                }
                Ok(())
            }
        }
    };
    (
        $(
            $type:ty
        ),*
    ) => {
        $(
            impl MessageSize for $type {
                const SIZE: usize = size_of::<$type>();
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
                fn encode(&self, data: &mut [u8]) -> Result<(), SerdeError> {
                    ensure_size!(data, Self);
                    let mut data = Cursor::new(data);
                    paste! {
                        data.[<write_ $type>]::<LE>(*self as _)?;
                    }
                    Ok(())
                }
            }
        )*
    };
}

pub trait Decode: MessageSize {
    fn decode(data: &[u8]) -> Result<Self, SerdeError>;
}

pub trait Encode: MessageSize {
    fn encode(&self, data: &mut [u8]) -> Result<(), SerdeError>;
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


#[derive(Debug, Error)]
pub enum SerdeError {
    #[error("The data provided is not long enough to decode the expected type.")]
    InvalidSize,
    #[error("IO error occurred while decoding")]
    IoError(#[from] std::io::Error),
}
