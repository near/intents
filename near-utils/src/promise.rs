use near_sdk::{Promise, env, json_types::U128, serde_json};

pub trait PromiseExt: Sized {
    fn and_maybe(self, p: Option<Promise>) -> Promise;
}

impl PromiseExt for Promise {
    #[inline]
    fn and_maybe(self, p: Option<Promise>) -> Promise {
        if let Some(p) = p { self.and(p) } else { self }
    }
}

pub const MAX_BOOL_JSON_LEN: usize = " false ".len();
pub const MAX_U128_LEN: usize = "+340282366920938463463374607431768211455".len();
pub const MAX_U128_JSON_LEN: usize = " \"\" ".len() + MAX_U128_LEN;

#[must_use]
pub const fn max_list_u128_json_len(count: usize) -> usize {
    const MAX_LEN_PER_AMOUNT: usize = "        \"\",\n".len() + MAX_U128_LEN;

    count
        .saturating_mul(MAX_LEN_PER_AMOUNT)
        .saturating_add("[\n]".len())
}

#[inline]
#[must_use]
pub fn promise_result_bool(result_idx: u64) -> Option<bool> {
    env::promise_result_checked(result_idx, MAX_BOOL_JSON_LEN)
        .ok()
        .and_then(|value| serde_json::from_slice::<bool>(&value).ok())
}

#[allow(non_snake_case)]
#[inline]
#[must_use]
pub fn promise_result_U128(result_idx: u64) -> Option<U128> {
    env::promise_result_checked(result_idx, MAX_U128_JSON_LEN)
        .ok()
        .and_then(|value| serde_json::from_slice::<U128>(&value).ok())
}

#[allow(non_snake_case)]
#[inline]
#[must_use]
pub fn promise_result_vec_U128(result_idx: u64, expected_len: usize) -> Option<Vec<U128>> {
    env::promise_result_checked(result_idx, max_list_u128_json_len(expected_len))
        .ok()
        .and_then(|value| serde_json::from_slice::<Vec<U128>>(&value).ok())
        .filter(|v| v.len() == expected_len)
}

#[cfg(test)]
mod tests {
    use near_sdk::json_types::U128;
    use rstest::rstest;

    use super::*;

    #[test]
    fn test_max_bool_json_len() {
        let prettified_false = serde_json::to_string_pretty(&false).unwrap();
        let prettified_true = serde_json::to_string_pretty(&true).unwrap();
        assert!(prettified_false.len() <= MAX_BOOL_JSON_LEN);
        assert!(prettified_true.len() <= MAX_BOOL_JSON_LEN);

        let compact_false = serde_json::to_string(&false).unwrap();
        let compact_true = serde_json::to_string(&true).unwrap();
        assert!(compact_false.len() <= MAX_BOOL_JSON_LEN);
        assert!(compact_true.len() <= MAX_BOOL_JSON_LEN);
    }

    #[test]
    fn test_max_u128_json_len() {
        let max_val = U128(u128::MAX);
        let prettified = serde_json::to_string_pretty(&max_val).unwrap();
        assert!(prettified.len() <= MAX_U128_JSON_LEN);

        let compact = serde_json::to_string(&max_val).unwrap();
        assert!(compact.len() <= MAX_U128_JSON_LEN);
    }

    #[rstest]
    #[case::len_0(0)]
    #[case::len_1(1)]
    #[case::len_2(2)]
    #[case::len_5(5)]
    #[case::len_10(10)]
    #[case::len_100(100)]
    fn test_max_list_u128_json_len(#[case] count: usize) {
        let vec: Vec<U128> = vec![U128(u128::MAX); count];
        let prettified = serde_json::to_string_pretty(&vec).unwrap();
        let max_len = max_list_u128_json_len(count);
        assert!(prettified.len() <= max_len);

        let compact = serde_json::to_string(&vec).unwrap();
        assert!(compact.len() <= max_len);
    }
}
