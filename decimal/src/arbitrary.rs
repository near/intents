use arbitrary::{Arbitrary, Error, Result, Unstructured};

use crate::UD128;

impl<'a> Arbitrary<'a> for UD128 {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let decimals = u.int_in_range(0..=Self::MAX_DECIMALS)?;
        let digits = u.arbitrary()?;
        Self::new(decimals, digits).ok_or(Error::IncorrectFormat)
    }

    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        const SIZE: usize = size_of::<u8>() + size_of::<u128>();
        (SIZE, Some(SIZE))
    }
}
