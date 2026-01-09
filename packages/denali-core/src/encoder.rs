use frunk::{HCons, HList, HNil, hlist, hlist::HList};

use crate::{
    id::IdFactory,
    message::{EncodeWithNewId, Event, MessageTypeMarker, OutgoingMessage, Request},
    wire::serde::{CompileTimeMessageSize, MessageSize},
};

pub trait EncodableList: HList + EncodeWithNewId {}

impl EncodableList for HNil {}
impl EncodeWithNewId for HNil {
    fn encode(
        &self,
        _data: &mut [u8],
        _id_factory: IdFactory<'_>,
    ) -> Result<usize, crate::wire::serde::SerdeError> {
        Ok(0)
    }
}
impl CompileTimeMessageSize for HNil {
    const SIZE: usize = 0;
}
impl MessageSize for HNil {
    fn size(&self) -> usize {
        Self::SIZE
    }
}

impl<H: EncodeWithNewId, T: EncodableList> EncodableList for HCons<H, T> {}
impl<H: EncodeWithNewId, T: EncodableList> EncodeWithNewId for HCons<H, T> {
    fn encode(
        &self,
        data: &mut [u8],
        id_factory: IdFactory<'_>,
    ) -> Result<usize, crate::wire::serde::SerdeError> {
        let mgr = id_factory.into_inner();

        let mut offset = self.head.encode(data, IdFactory::new(mgr))?;
        offset += self.tail.encode(&mut data[offset..], IdFactory::new(mgr))?;

        Ok(offset)
    }
}
impl<H: EncodeWithNewId, T: EncodableList> MessageSize for HCons<H, T> {
    fn size(&self) -> usize {
        self.head.size() + self.tail.size()
    }
}
impl<H: EncodeWithNewId + CompileTimeMessageSize, T: EncodableList + CompileTimeMessageSize>
    CompileTimeMessageSize for HCons<H, T>
{
    const SIZE: usize = H::SIZE + T::SIZE;
}

pub trait IntoRawEncodableList<M: MessageTypeMarker>: OutgoingMessage<M> {
    type Output: EncodableList;
    fn into_raw_encodable_list(self) -> Self::Output;
}

pub struct Encoder<M: MessageTypeMarker, L: EncodableList> {
    messages: L,

    _marker: std::marker::PhantomData<M>,
}

impl<M: MessageTypeMarker> Encoder<M, HNil> {
    pub const fn new() -> Self {
        Self {
            messages: HNil,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<L: EncodableList> Encoder<Request, L> {
    pub fn request<R: IntoRawEncodableList<Request>>(
        self,
        request: R,
    ) -> Encoder<Request, HList!(R::Output, ...L)> {
        Encoder {
            messages: hlist![request.into_raw_encodable_list(), ...self.messages],
            _marker: std::marker::PhantomData,
        }
    }
}
impl<L: EncodableList> Encoder<Event, L> {
    pub fn event<E: IntoRawEncodableList<Request>>(
        self,
        event: E,
    ) -> Encoder<Event, HList!(E::Output, ...L)> {
        Encoder {
            messages: hlist![event.into_raw_encodable_list(), ...self.messages],
            _marker: std::marker::PhantomData,
        }
    }
}

impl<M: MessageTypeMarker> Default for Encoder<M, HNil> {
    fn default() -> Self {
        Self::new()
    }
}
