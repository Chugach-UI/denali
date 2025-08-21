use std::io::Cursor;

pub mod serde;

fn round_up_4(pos: u64) -> u64 {
    (pos + 3) & !3
}

pub struct MessageTraverser<'a> {
    data: Cursor<&'a mut [u8]>,
}
impl<'a> MessageTraverser<'a> {
    pub fn new(data: &'a mut [u8]) -> Self {
        Self {
            data: Cursor::new(data),
        }
    }

    pub fn read<T: serde::Decode>(&mut self) -> Result<T, serde::SerdeError> {
        let pos = self.position();
        let data = &self.data.get_ref()[pos as usize..];

        let result = T::decode(data)?;
        self.data
            .set_position(round_up_4(self.data.position() + result.size() as u64));
        Ok(result)
    }
    pub fn write<T: serde::Encode>(&mut self, value: &T) -> Result<(), serde::SerdeError> {
        let pos = self.position();
        let data = &mut self.data.get_mut()[pos as usize..];

        value.encode(data)?;
        self.data
            .set_position(round_up_4(self.data.position() + value.size() as u64));
        Ok(())
    }

    #[inline]
    pub fn set_position(&mut self, pos: u64) {
        self.data.set_position(pos);
    }
    #[inline]
    pub fn position(&self) -> u64 {
        self.data.position()
    }
}

#[cfg(test)]
mod tests {
    use crate::wire::serde::Array;

    use super::MessageTraverser;

    #[test]
    fn test_message_traverser() {
        let mut buffer = [0u8; 64];
        let mut traverser = MessageTraverser::new(&mut buffer);

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
