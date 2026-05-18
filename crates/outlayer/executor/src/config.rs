use defuse_outlayer_vm_runner::VmRuntime;

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize)
)]
pub struct ExecutorConfig {
    #[cfg_attr(
        feature = "serde",
        serde_as(as = "Clamp<{ 1024 * 1024 }, { 4 * 1024 * 1024 * 1024 }, usize>")
    )]
    pub memory_limit: usize,

    #[cfg_attr(feature = "serde", serde_as(as = "Clamp<1, { 64 * 1024 * 1024 }, usize>"))]
    pub stdin_limit: usize,

    #[cfg_attr(feature = "serde", serde_as(as = "Clamp<1, { 64 * 1024 * 1024 }, usize>"))]
    pub stdout_limit: usize,

    #[cfg_attr(feature = "serde", serde_as(as = "Clamp<1, { 1024 * 1024 }, usize>"))]
    pub stderr_limit: usize,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        const STDIN_LIMIT: usize = 4 * 1024 * 1024; // 4 MB
        const STDOUT_LIMIT: usize = 4 * 1024 * 1024; // 4 MB
        const STDERR_LIMIT: usize = 16 * 1024; // 16 KB
        const MEMORY_LIMIT: usize = 100 * 1024 * 1024; // 100 MB

        Self {
            stdin_limit: STDIN_LIMIT,
            stdout_limit: STDOUT_LIMIT,
            stderr_limit: STDERR_LIMIT,
            memory_limit: MEMORY_LIMIT,
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
