use near_sdk::{Promise, env, json_types::U128, serde::de::DeserializeOwned, serde_json};

pub trait PromiseExt: Sized {
    fn and_maybe(self, p: Option<Promise>) -> Promise;
}

impl PromiseExt for Promise {
    #[inline]
    fn and_maybe(self, p: Option<Promise>) -> Promise {
        if let Some(p) = p { self.and(p) } else { self }
    }
}

pub type PromiseResult<T> = Result<T, near_sdk::PromiseError>;
pub type PromiseJsonResult<T> = Result<Result<T, serde_json::Error>, near_sdk::PromiseError>;

pub const MAX_JSON_LENGTH_RECURSION_LIMIT: usize = 1024;

pub trait MaxJsonLength: DeserializeOwned {
    type Args;

    fn max_json_length(args: Self::Args) -> usize {
        Self::max_json_length_at_depth(0, args)
    }

    fn max_json_length_at_depth(depth: usize, args: Self::Args) -> usize;
}

#[inline]
pub fn promise_result_checked_json_with_args<T: MaxJsonLength>(
    result_idx: u64,
    args: T::Args,
) -> PromiseJsonResult<T> {
    let value = env::promise_result_checked(result_idx, T::max_json_length(args))?;
    Ok(serde_json::from_slice::<T>(&value))
}

#[inline]
pub fn promise_result_checked_json<T: MaxJsonLength<Args = ()>>(
    result_idx: u64,
) -> PromiseJsonResult<T> {
    promise_result_checked_json_with_args::<T>(result_idx, ())
}

#[inline]
pub fn promise_result_checked_json_with_len<T: MaxJsonLength<Args = (usize, ())>>(
    result_idx: u64,
    length: usize,
) -> PromiseJsonResult<T> {
    let max_len = T::max_json_length((length, ()));
    let value = env::promise_result_checked(result_idx, max_len)?;
    Ok(serde_json::from_slice::<T>(&value))
}

/// Returns `Ok(())` if the promise at `result_idx` succeeded with an empty result.
/// This is the expected outcome for void-returning cross-contract calls
/// (e.g. `ft_transfer`, `nft_transfer`, `mt_batch_transfer`).
#[inline]
pub fn promise_result_checked_void(result_idx: u64) -> PromiseResult<()> {
    let data = env::promise_result_checked(result_idx, 0)?;
    debug_assert!(data.is_empty());
    Ok(())
}

impl MaxJsonLength for bool {
    type Args = ();

    fn max_json_length_at_depth(_depth: usize, _args: ()) -> usize {
        " false ".len()
    }
}

impl MaxJsonLength for U128 {
    type Args = ();

    fn max_json_length_at_depth(_depth: usize, _args: ()) -> usize {
        " \"\" ".len() + "+340282366920938463463374607431768211455".len()
    }
}

impl<T> MaxJsonLength for Vec<T>
where
    T: MaxJsonLength,
{
    type Args = (usize, T::Args);

    fn max_json_length_at_depth(depth: usize, (length, inner_args): (usize, T::Args)) -> usize {
        if depth >= MAX_JSON_LENGTH_RECURSION_LIMIT {
            return usize::MAX;
        }

        let ident = "        ".len().saturating_mul(depth + 1);

        ident
            .saturating_add(T::max_json_length_at_depth(depth + 1, inner_args))
            .saturating_add(",\n".len())
            .saturating_mul(length)
            .saturating_add(" [\n] ".len() + ident)
    }
}

impl<T, const N: usize> MaxJsonLength for [T; N]
where
    T: MaxJsonLength,
    Self: DeserializeOwned,
{
    type Args = T::Args;

    fn max_json_length_at_depth(depth: usize, args: Self::Args) -> usize {
        <Vec<T>>::max_json_length_at_depth(depth, (N, args))
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::json_types::U128;
    use rstest::rstest;

    use super::*;

    #[test]
    fn test_max_bool_json_len() {
        let max_len = bool::max_json_length(());

        let prettified_false = serde_json::to_string_pretty(&false).unwrap();
        let prettified_true = serde_json::to_string_pretty(&true).unwrap();
        assert!(prettified_false.len() <= max_len);
        assert!(prettified_true.len() <= max_len);

        let compact_false = serde_json::to_string(&false).unwrap();
        let compact_true = serde_json::to_string(&true).unwrap();
        assert!(compact_false.len() <= max_len);
        assert!(compact_true.len() <= max_len);
    }

    #[test]
    fn test_max_u128_json_len() {
        let max_len = U128::max_json_length(());

        let max_val = U128(u128::MAX);
        let prettified = serde_json::to_string_pretty(&max_val).unwrap();
        assert!(prettified.len() <= max_len);

        let compact = serde_json::to_string(&max_val).unwrap();
        assert!(compact.len() <= max_len);
    }

    #[rstest]
    #[case::len_0(0)]
    #[case::len_1(1)]
    #[case::len_2(2)]
    #[case::len_5(5)]
    #[case::len_10(10)]
    #[case::len_100(100)]
    fn test_max_vec_u128_json_len(#[case] count: usize) {
        let max_len = Vec::<U128>::max_json_length((count, ()));

        let vec: Vec<U128> = vec![U128(u128::MAX); count];
        let prettified = serde_json::to_string_pretty(&vec).unwrap();
        assert!(prettified.len() <= max_len);

        let compact = serde_json::to_string(&vec).unwrap();
        assert!(compact.len() <= max_len);
    }

    #[rstest]
    #[case::outer_1_inner_3(1, 3)]
    #[case::outer_3_inner_5(3, 5)]
    #[case::outer_5_inner_10(5, 10)]
    fn test_max_nested_vec_u128_json_len(#[case] outer: usize, #[case] inner: usize) {
        let max_len = Vec::<Vec<U128>>::max_json_length_at_depth(0, (outer, (inner, ())));

        let vec: Vec<Vec<U128>> = vec![vec![U128(u128::MAX); inner]; outer];
        let prettified = serde_json::to_string_pretty(&vec).unwrap();
        assert!(prettified.len() <= max_len);

        let compact = serde_json::to_string(&vec).unwrap();
        assert!(compact.len() <= max_len);
    }

    #[test]
    fn test_max_array_u128_json_len() {
        let max_len = <[U128; 5]>::max_json_length(());

        let arr = [U128(u128::MAX); 5];
        let prettified = serde_json::to_string_pretty(&arr).unwrap();
        assert!(prettified.len() <= max_len);

        let compact = serde_json::to_string(&arr).unwrap();
        assert!(compact.len() <= max_len);
    }

    #[test]
    fn test_depth_exceeds_limit_returns_max() {
        assert_eq!(
            Vec::<U128>::max_json_length_at_depth(MAX_JSON_LENGTH_RECURSION_LIMIT + 1, (10, ())),
            usize::MAX,
        );
    }
}
