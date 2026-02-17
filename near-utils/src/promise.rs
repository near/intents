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

pub trait MaxJsonLength: DeserializeOwned {
    type Args;

    /// Default: starts with remaining depth of 1 (sufficient for non-nested types)
    fn max_json_length(args: Self::Args) -> usize {
        Self::max_json_length_at_depth(args, 1)
    }

    fn max_json_length_at_depth(args: Self::Args, remaining_depth: usize) -> usize;
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

/// Returns `Ok(())` if the promise at `result_idx` succeeded with an empty result.
/// This is the expected outcome for void-returning cross-contract calls
/// (e.g. `ft_transfer`, `nft_transfer`, `mt_batch_transfer`).
#[inline]
pub fn promise_result_checked_void(result_idx: u64) -> PromiseResult<()> {
    let data = env::promise_result_checked(result_idx, 0)?;
    if data.is_empty() {
        Ok(())
    } else {
        unreachable!()
    }
}

impl MaxJsonLength for bool {
    type Args = ();

    fn max_json_length_at_depth(_args: (), _remaining_depth: usize) -> usize {
        " false ".len()
    }
}

impl MaxJsonLength for U128 {
    type Args = ();

    fn max_json_length_at_depth(_args: (), _remaining_depth: usize) -> usize {
        " \"\" ".len() + "+340282366920938463463374607431768211455".len()
    }
}

impl<T> MaxJsonLength for Vec<T>
where
    T: MaxJsonLength,
{
    type Args = (usize, T::Args);

    fn max_json_length_at_depth((length, args): (usize, T::Args), remaining_depth: usize) -> usize {
        if remaining_depth == 0 {
            return usize::MAX;
        }

        let ident = "        ".len().saturating_mul(remaining_depth + 1);

      ident
            .saturating_add(T::max_json_length_at_depth(args, remaining_depth + 1))
            .saturating_add(",\n".len())
            .saturating_mul(length)
            .saturating_add(" [\n ".len() + ident + "]".len())

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
        let max_len =
            Vec::<Vec<U128>>::max_json_length_at_depth((outer, (inner, ())), 2);

        let vec: Vec<Vec<U128>> = vec![vec![U128(u128::MAX); inner]; outer];
        let prettified = serde_json::to_string_pretty(&vec).unwrap();
        assert!(prettified.len() <= max_len);

        let compact = serde_json::to_string(&vec).unwrap();
        assert!(compact.len() <= max_len);
    }

    #[test]
    fn test_depth_zero_returns_max() {
        assert_eq!(
            Vec::<U128>::max_json_length_at_depth((10, ()), 0),
            usize::MAX,
        );
    }
}
