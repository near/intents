use near_sdk::{
    base64::engine::{Engine, general_purpose::STANDARD},
    borsh::{self, BorshSerialize, io},
};

pub struct Base64;

impl Base64 {
    #[inline]
    pub fn serialize<T, W>(obj: &T, writer: &mut W) -> io::Result<()>
    where
        T: AsRef<[u8]>,
        W: io::Write,
    {
        let s = STANDARD.encode(obj.as_ref());
        s.serialize(writer)
    }

    #[inline]
    pub fn deserialize<T, R>(reader: &mut R) -> io::Result<T>
    where
        T: TryFrom<Vec<u8>>,
        R: io::Read,
    {
        let s: String = borsh::BorshDeserialize::deserialize_reader(reader)?;
        let v = STANDARD.decode(s).map_err(io::Error::other)?;
        let length = v.len();
        v.try_into().map_err(|_| {
            io::Error::other(format!(
                "can't convert a byte vector of length {length} into the output type"
            ))
        })
    }
}
