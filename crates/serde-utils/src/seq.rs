use serde::{Serialize, Serializer};
use serde_with::SerializeAs;

/// An adaptor for serializing a sequence backwards
///
/// # Examples
///
/// ```rust
/// # use serde::Serialize;
/// # use serde_json::json;
/// use defuse_serde_utils::Reversed;
/// use serde_with::serde_as;
///
/// #[serde_as]
/// #[derive(Serialize)]
/// struct A {
///     #[serde_as(as = "Reversed")]
///     reversed: Vec<u32>,
/// }
///
/// assert_eq!(
///     serde_json::to_value(&A { reversed: vec![3, 2, 1] }).unwrap(),
///     json!({ "reversed": [1, 2, 3] }),
/// );
/// ```
pub struct Reversed;

impl<T> SerializeAs<[T]> for Reversed
where
    T: Serialize,
{
    #[inline]
    fn serialize_as<S>(source: &[T], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(source.iter().rev())
    }
}
