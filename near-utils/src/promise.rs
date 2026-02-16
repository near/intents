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
    fn max_json_length(args: Self::Args) -> usize;
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

    fn max_json_length(_args: ()) -> usize {
        " false ".len()
    }
}

impl MaxJsonLength for U128 {
    type Args = ();

    fn max_json_length(_args: ()) -> usize {
        " \"\" ".len() + "+340282366920938463463374607431768211455".len()
    }
}

impl<T> MaxJsonLength for Vec<T>
where
    T: MaxJsonLength<Args = ()>,
{
    type Args = usize;

    fn max_json_length(length: usize) -> usize {
        // account for ',' '\n' and identation in prettified format
        const PER_ITEM_OVERHEAED: usize = "        ,\n".len();
        let single_elem_max_length = T::max_json_length(()).saturating_add(PER_ITEM_OVERHEAED);

        length
            .saturating_mul(single_elem_max_length)
            .saturating_add("\n[\n]".len())
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
        let max_len = Vec::<U128>::max_json_length(count);

        let vec: Vec<U128> = vec![U128(u128::MAX); count];
        let prettified = serde_json::to_string_pretty(&vec).unwrap();
        assert!(prettified.len() <= max_len);

        let compact = serde_json::to_string(&vec).unwrap();
        assert!(compact.len() <= max_len);
    }
}
