use arbitrary::Arbitrary;

/// Wrapper type to be able to generate arbitrary Arrays
#[derive(Debug, Clone, Copy, derive_more::From)]
pub struct Array<T, const N: usize>(pub [T; N]);

impl<T, const N: usize> From<Array<T, N>> for [T; N] {
    fn from(array: Array<T, N>) -> Self {
        array.0
    }
}

impl<'a, T: Arbitrary<'a> + Default + Copy, const N: usize> Arbitrary<'a> for Array<T, N> {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let mut array = [T::default(); N];
        for el in &mut array {
            *el = T::arbitrary(u)?;
        }

        Ok(Self(array))
    }
}
