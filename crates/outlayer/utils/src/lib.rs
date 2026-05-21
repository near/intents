#[cfg(feature = "serde")]
pub struct Clamp<const MIN: i64, const MAX: i64, T>(::core::marker::PhantomData<T>);

#[cfg(feature = "serde")]
impl<const MIN: i64, const MAX: i64, T: ::serde::Serialize> ::serde_with::SerializeAs<T>
    for Clamp<MIN, MAX, T>
{
    fn serialize_as<S: ::serde::Serializer>(source: &T, serializer: S) -> Result<S::Ok, S::Error> {
        source.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, const MIN: i64, const MAX: i64, T> ::serde_with::DeserializeAs<'de, T>
    for Clamp<MIN, MAX, T>
where
    T: TryFrom<i64>,
    T::Error: ::core::fmt::Display,
{
    fn deserialize_as<D: ::serde::Deserializer<'de>>(d: D) -> Result<T, D::Error> {
        use ::serde::Deserialize as _;
        let v = i64::deserialize(d)?.clamp(MIN, MAX);
        T::try_from(v).map_err(::serde::de::Error::custom)
    }
}
