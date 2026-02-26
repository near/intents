use core::marker::PhantomData;

use near_sdk::borsh::io::{self};
use tlbits::{
    NoArgs,
    adapters::Io,
    de::{BitReaderExt, BitUnpackAs},
    ser::{BitPackAs, BitWriterExt},
};

use crate::adapters::{BorshDeserializeAs, BorshSerializeAs};

pub struct Bits<As: ?Sized = tlbits::Same>(PhantomData<As>);

impl<T, As> BorshSerializeAs<T> for Bits<As>
where
    As: BitPackAs<T, Args: NoArgs> + ?Sized,
{
    fn serialize_as<W>(source: &T, writer: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        let mut w = Io::new(writer);
        w.pack_as::<_, &As>(source, NoArgs::EMPTY)?;
        w.stop_and_flush()?;
        Ok(())
    }
}

impl<T, As> BorshDeserializeAs<T> for Bits<As>
where
    As: for<'de> BitUnpackAs<'de, T, Args: NoArgs> + ?Sized,
{
    fn deserialize_as<R>(reader: &mut R) -> io::Result<T>
    where
        R: io::Read,
    {
        let mut r = Io::new(reader);
        let v = r.unpack_as::<_, As>(NoArgs::EMPTY)?;
        r.checked_discard()?;
        Ok(v)
    }
}
