#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize)
)]
#[derive(Debug, Clone, Copy)]
pub struct ExecutorLimits {
    #[cfg_attr(feature = "serde", serde_as(as = "Clamp<1, { 64 * 1024 * 1024 }, usize>"))]
    pub stdin: usize,

    #[cfg_attr(feature = "serde", serde_as(as = "Clamp<1, { 64 * 1024 * 1024 }, usize>"))]
    pub stdout: usize,

    #[cfg_attr(feature = "serde", serde_as(as = "Clamp<1, { 1024 * 1024 }, usize>"))]
    pub stderr: usize,
}

impl Default for ExecutorLimits {
    fn default() -> Self {
        Self {
            stdin: 4 * 1024 * 1024,
            stdout: 4 * 1024 * 1024,
            stderr: 16 * 1024,
        }
    }
}

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize)
)]
pub struct Config {
    #[cfg_attr(
        feature = "serde",
        serde_as(as = "Clamp<{ 1024 * 1024 }, { 4 * 1024 * 1024 * 1024 }, usize>")
    )]
    pub memory_limit: usize,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub limits: ExecutorLimits,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            memory_limit: 100 * 1024 * 1024,
            limits: ExecutorLimits::default(),
        }
    }
}

//TODO: is that even required, given measurements?
#[cfg(feature = "serde")]
struct Clamp<const MIN: i64, const MAX: i64, T>(::core::marker::PhantomData<T>);

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
