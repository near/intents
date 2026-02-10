pub fn is_default<T>(value: &T) -> bool
where
    T: Default + PartialEq,
{
    value == &T::default()
}

#[cfg(any(feature = "arbitrary", test))]
pub mod arbitrary {
    use ::arbitrary::{Result, Unstructured};
    use near_sdk::{Gas, NearToken};

    pub fn near_token(u: &mut Unstructured<'_>) -> Result<NearToken> {
        u.int_in_range(NearToken::ZERO.as_yoctonear()..=NearToken::MAX.as_yoctonear())
            .map(NearToken::from_yoctonear)
    }

    pub fn gas(u: &mut Unstructured<'_>) -> Result<Gas> {
        u.int_in_range(0..=Gas::from_tgas(300).as_gas())
            .map(Gas::from_gas)
    }
}
