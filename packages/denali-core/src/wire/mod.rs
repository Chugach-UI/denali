use std::io::Cursor;

use serde::CompileTimeMessageSize;

pub mod serde;

/// Pads the given position to the next multiple of 4 bytes (32 bits).
#[must_use]
pub const fn pad_to_32_bits(pos: usize) -> usize {
    (pos + 3) & !3
}

/// A helper for decoding byte buffers from the Wayland wire protocol.
pub struct MessageDecoder<'a> {
    data: Cursor<&'a [u8]>,
}
impl<'a> MessageDecoder<'a> {
    /// Creates a new `MessageDecoder` for the given byte slice.
    #[must_use]
    pub const fn new(data: &'a [u8]) -> Self {
        Self {
            data: Cursor::new(data),
        }
    }

    /// Reads a value of type `T` from the current position in the byte buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if decoding fails. See [`Decode::decode`](serde::Decode::decode) for more details.
    pub fn read<T: serde::Decode>(&mut self) -> Result<T, serde::SerdeError> {
        let pos = self.position();
        let data = &self.data.get_ref()[pos as usize..];

        let result = T::decode(data)?;
        self.data
            .set_position(pad_to_32_bits(self.data.position() as usize + result.size()) as _);
        Ok(result)
    }

    /// Sets the current position in the byte buffer.
    #[inline]
    pub const fn set_position(&mut self, pos: u64) {
        self.data.set_position(pos);
    }
    /// Returns the current position in the byte buffer.
    #[inline]
    #[must_use]
    pub const fn position(&self) -> u64 {
        self.data.position()
    }

    /// Returns a reference to the underlying byte slice.
    #[inline]
    #[must_use]
    pub const fn get_ref(&self) -> &[u8] {
        self.data.get_ref()
    }
}

/// A helper for encoding or decoding byte buffers for the Wayland wire protocol.
pub struct MessageEncoder<'a> {
    data: Cursor<&'a mut [u8]>,
}
impl<'a> MessageEncoder<'a> {
    /// Creates a new `MessageEncoder` for the given mutable byte slice.
    pub const fn new(data: &'a mut [u8]) -> Self {
        Self {
            data: Cursor::new(data),
        }
    }

    /// Reads a value of type `T` from the current position in the byte buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if decoding fails. See [`Decode::decode`](serde::Decode::decode) for more details.
    pub fn read<T: serde::Decode>(&mut self) -> Result<T, serde::SerdeError> {
        let pos = self.position();
        let data = &self.data.get_ref()[pos as usize..];

        let result = T::decode(data)?;
        self.data
            .set_position(pad_to_32_bits(self.data.position() as usize + result.size()) as _);
        Ok(result)
    }
    /// Writes a value of type `T` to the current position in the byte buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if encoding fails. See [`Encode::encode`](serde::Encode::encode) for more details.
    pub fn write<T: serde::Encode>(&mut self, value: &T) -> Result<(), serde::SerdeError> {
        let pos = self.position();
        let data = &mut self.data.get_mut()[pos as usize..];

        value.encode(data)?;
        self.data
            .set_position(pad_to_32_bits(self.data.position() as usize + value.size()) as _);
        Ok(())
    }

    /// Sets the current position in the byte buffer.
    #[inline]
    pub const fn set_position(&mut self, pos: u64) {
        self.data.set_position(pos);
    }
    /// Returns the current position in the byte buffer.
    #[inline]
    #[must_use]
    pub const fn position(&self) -> u64 {
        self.data.position()
    }

    /// Returns a reference to the underlying byte slice.
    #[inline]
    #[must_use]
    pub const fn get_ref(&self) -> &[u8] {
        self.data.get_ref()
    }
}

/// Encodes a message with the given object ID and opcode into the provided byte buffer.
///
/// # Errors
///
/// Returns an error if encoding fails. See [`Encode::encode`](serde::Encode::encode) for more details.
pub fn encode_message<T: serde::Encode>(
    message: &T,
    object_id: u32,
    opcode: u16,
    data: &mut [u8],
) -> Result<usize, serde::SerdeError> {
    let mut traverser = MessageEncoder::new(data);
    let header = serde::MessageHeader {
        object_id,
        size: (serde::MessageHeader::SIZE + message.size()) as u16,
        opcode,
    };

    traverser.write(&header)?;
    traverser.write(message)?;

    Ok(traverser.position() as usize)
}

#[cfg(test)]
mod tests {
    extern crate test;

    use crate::wire::serde::Array;

    use super::MessageEncoder;

    #[bench]
    fn bench_message_traverser_write(b: &mut test::Bencher) {
        let mut buffer = [0u8; 64];
        let mut traverser = MessageEncoder::new(&mut buffer);

        b.iter(|| {
            traverser
                .write(&super::serde::MessageHeader {
                    object_id: 1,
                    size: 16,
                    opcode: 3,
                })
                .unwrap();
            traverser.write(&8i32).unwrap();
            traverser.write(&19u32).unwrap();
            traverser.write::<Array>(&[4u8; 4].into()).unwrap();
            traverser
                .write::<super::serde::String>(&"test".into())
                .unwrap();
            traverser.set_position(0);
        });
    }
    #[bench]
    fn bench_message_traverser_read(b: &mut test::Bencher) {
        let mut buffer = [
            1, 0, 0, 0, 16, 0, 3, 0, 8, 0, 0, 0, 19, 0, 0, 0, 4, 0, 0, 0, 4, 4, 4, 4, 4, 0, 0, 0,
            116, 101, 115, 116, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let mut traverser = MessageEncoder::new(&mut buffer);

        b.iter(|| {
            let header: super::serde::MessageHeader = traverser.read().unwrap();
            let value_i32: i32 = traverser.read().unwrap();
            let value_u32: u32 = traverser.read().unwrap();
            let array: Array = traverser.read().unwrap();
            let string: super::serde::String = traverser.read().unwrap();
            traverser.set_position(0);
        });
    }

    #[test]
    fn test_message_traverser() {
        let mut buffer = [0u8; 64];
        let mut traverser = MessageEncoder::new(&mut buffer);

        // test encoding
        traverser
            .write(&super::serde::MessageHeader {
                object_id: 1,
                size: 16,
                opcode: 3,
            })
            .unwrap();
        traverser.write(&8i32).unwrap();
        traverser.write(&19u32).unwrap();
        traverser.write::<Array>(&[4u8; 4].into()).unwrap();
        traverser
            .write::<super::serde::String>(&"test".into())
            .unwrap();

        // test decoding
        traverser.set_position(0);
        let header: super::serde::MessageHeader = traverser.read().unwrap();
        assert_eq!(header.object_id, 1);
        assert_eq!(header.size, 16);
        assert_eq!(header.opcode, 3);
        let value_i32: i32 = traverser.read().unwrap();
        assert_eq!(value_i32, 8);
        let value_u32: u32 = traverser.read().unwrap();
        assert_eq!(value_u32, 19);
        let array: Array = traverser.read().unwrap();
        assert_eq!(array.data.len(), 4);
        assert_eq!(&*array.data, &[4u8; 4]);
        let string: super::serde::String = traverser.read().unwrap();
        assert_eq!(string.data, "test");
    }
}
