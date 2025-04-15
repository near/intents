use std::{io, marker::PhantomData};

pub trait BorshSerializeAs<T: ?Sized> {
    fn serialize_as<S, W>(source: &S, writer: &mut W) -> io::Result<()>
    where
        W: io::Write;
}

pub trait BorshDeserializeAs<'de, T>: Sized {
    fn deserialize_as<R>(reader: &mut R) -> io::Result<T>
    where
        R: io::Read;
}

pub struct As<T: ?Sized>(PhantomData<T>);

impl<T: ?Sized> As<T> {
    pub fn serialize<S, W>(obj: &T, writer: &mut W)
}
