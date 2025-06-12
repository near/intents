use super::*;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use std::io::Cursor;

// Helper roundtrip
fn roundtrip<T: BorshSerialize + BorshDeserialize + PartialEq + std::fmt::Debug>(val: &T) {
    let data = borsh::to_vec(&val).unwrap();
    let de: T = BorshDeserialize::try_from_slice(&data).unwrap();
    assert_eq!(&de, val);
}

#[test]
fn test_same_identity() {
    let val: u32 = 42;
    let mut buf = vec![];
    Same::serialize_as(&val, &mut buf).unwrap();
    let deser: u32 = Same::deserialize_as(&mut Cursor::new(&buf)).unwrap();
    assert_eq!(val, deser);
}

#[test]
fn test_aswrap_same() {
    let val = AsWrap::<_, Same>::new(1337u32);
    let data = borsh::to_vec(&val).unwrap();
    let restored: AsWrap<u32, Same> = BorshDeserialize::try_from_slice(&data).unwrap();
    assert_eq!(val.into_inner(), restored.into_inner());
}

#[derive(Debug, Clone, PartialEq)]
struct MyInt(i64);

impl From<MyInt> for i32 {
    fn from(val: MyInt) -> Self {
        i32::try_from(val.0).unwrap()
    }
}

impl From<&MyInt> for i32 {
    fn from(val: &MyInt) -> Self {
        i32::try_from(val.0).unwrap()
    }
}

impl From<i32> for MyInt {
    fn from(val: i32) -> Self {
        MyInt(i64::from(val))
    }
}

#[test]
fn test_from_into_strategy() {
    let val = MyInt(123);
    let mut buf = vec![];
    FromInto::<i32>::serialize_as(&val, &mut buf).unwrap();
    let restored = FromInto::<i32>::deserialize_as(&mut Cursor::new(&buf)).unwrap();
    assert_eq!(val, restored);
}

#[test]
fn test_from_into_ref_strategy() {
    let val = MyInt(456);
    let mut buf = vec![];
    FromIntoRef::<i32>::serialize_as(&val, &mut buf).unwrap();
    let restored = FromIntoRef::<i32>::deserialize_as(&mut Cursor::new(&buf)).unwrap();
    assert_eq!(val, restored);
}

#[derive(Debug, Clone, PartialEq)]
struct OldFormat(u8);
#[derive(Debug, Clone, PartialEq)]
struct NewFormat(u8);

impl BorshDeserialize for OldFormat {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        Ok(OldFormat(u8::deserialize_reader(reader)?))
    }
}

impl BorshDeserialize for NewFormat {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        Ok(NewFormat(u8::deserialize_reader(reader)?))
    }
}

impl BorshDeserializeAs<u8> for OldFormat {
    fn deserialize_as<R: io::Read>(reader: &mut R) -> io::Result<u8> {
        // read OldFormat, then extract its inner
        let of = OldFormat::deserialize_reader(reader)?;
        Ok(of.0)
    }
}

impl BorshDeserializeAs<u8> for NewFormat {
    fn deserialize_as<R: io::Read>(reader: &mut R) -> io::Result<u8> {
        let nf = NewFormat::deserialize_reader(reader)?;
        Ok(nf.0)
    }
}

impl From<OldFormat> for u8 {
    fn from(o: OldFormat) -> Self {
        o.0
    }
}
impl From<NewFormat> for u8 {
    fn from(n: NewFormat) -> Self {
        n.0
    }
}

#[test]
fn test_or_fallback() {
    let buf = borsh::to_vec(&99u8).unwrap();
    let val = Or::<OldFormat, NewFormat>::deserialize_as(&mut Cursor::new(&buf)).unwrap();
    assert_eq!(val, 99);
}

#[test]
fn test_option_wrap() {
    let opt = Some(42u32);
    let mut buf = vec![];
    <Option<Same> as BorshSerializeAs<Option<u32>>>::serialize_as(&opt, &mut buf).unwrap();
    let output =
        <Option<Same> as BorshDeserializeAs<Option<u32>>>::deserialize_as(&mut Cursor::new(&buf))
            .unwrap();
    assert_eq!(opt, output);
}

#[test]
fn test_box_rc_arc() {
    let boxed = AsWrap::<Box<u64>, Same>::new(Box::new(100));
    roundtrip(&boxed);

    // These don't work because they need `rc` feature in borsh
    // let rc = AsWrap::<Rc<u64>, Same>::new(Rc::new(200));
    // roundtrip(&rc);

    // let arc = AsWrap::<Arc<u64>, Same>::new(Arc::new(300));
    // roundtrip(&arc);
}

#[test]
fn test_array_tuple() {
    let arr = [1u8, 2, 3];
    let mut buf = vec![];
    <[Same; 3] as BorshSerializeAs<[u8; 3]>>::serialize_as(&arr, &mut buf).unwrap();
    let out =
        <[Same; 3] as BorshDeserializeAs<[u8; 3]>>::deserialize_as(&mut Cursor::new(&buf)).unwrap();
    assert_eq!(arr, out);

    let tup = (10u8, 20u16);
    let mut buf = vec![];
    <(Same, Same) as BorshSerializeAs<(u8, u16)>>::serialize_as(&tup, &mut buf).unwrap();
    let out =
        <(Same, Same) as BorshDeserializeAs<(u8, u16)>>::deserialize_as(&mut Cursor::new(&buf))
            .unwrap();
    assert_eq!(tup, out);
}
