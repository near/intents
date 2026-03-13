use core::{fmt::Display, str::FromStr};

use near_sdk::borsh::{self, BorshSerialize, io};

pub struct DisplayFromStr;

impl DisplayFromStr {
    #[inline]
    pub fn serialize<T, W>(obj: &T, writer: &mut W) -> io::Result<()>
    where
        T: Display,
        W: io::Write,
    {
        obj.to_string().serialize(writer)
    }

    #[inline]
    pub fn deserialize<T, R>(reader: &mut R) -> io::Result<T>
    where
        T: FromStr,
        T::Err: Display,
        R: io::Read,
    {
        let s: String = borsh::BorshDeserialize::deserialize_reader(reader)?;
        T::from_str(&s).map_err(|e| io::Error::other(e.to_string()))
    }
}
