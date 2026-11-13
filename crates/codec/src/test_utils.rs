use std::{borrow::Borrow, fmt::Debug};

use crate::{IoDecoder, IoEncoder};

#[allow(dead_code)]
pub fn assert_roundtrip<C, T>(value: &T, expected: Option<&[u8]>)
where
    T: ToOwned + PartialEq + Debug + ?Sized,
    C: IoEncoder<T> + IoDecoder<T::Owned>,
{
    let encoded = C::to_vec(value).expect("encode");
    if let Some(expected) = expected {
        assert_eq!(encoded, expected, "invalid encoded bytes");
    }
    let decoded = C::from_slice(encoded).expect("decode");
    assert_eq!(decoded.borrow(), value, "invalid decoded value");
}
