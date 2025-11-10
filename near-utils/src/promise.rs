use std::{iter::FusedIterator, ops::Range};

use near_sdk::{CryptoHash, Gas, GasWeight, PromiseIndex, PromiseResult, env, sys};

#[derive(Debug, Clone)]
pub struct PromiseResults(Range<u64>);

impl PromiseResults {
    #[inline]
    pub fn new() -> Self {
        Self(0..env::promise_results_count())
    }

    #[inline]
    pub const fn next_promise_index(&self) -> u64 {
        self.0.start
    }
}

impl Iterator for PromiseResults {
    type Item = PromiseResult;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(env::promise_result)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth(n).map(env::promise_result)
    }
}

impl ExactSizeIterator for PromiseResults {
    #[inline]
    fn len(&self) -> usize {
        self.0.end.try_into().unwrap_or_else(|_| unreachable!())
    }
}

impl FusedIterator for PromiseResults {}

impl DoubleEndedIterator for PromiseResults {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(env::promise_result)
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth_back(n).map(env::promise_result)
    }
}

pub type YieldId = CryptoHash;

pub fn promise_yield_create(
    function_name: &str,
    arguments: &[u8],
    gas: Gas,
    weight: GasWeight,
) -> (PromiseIndex, YieldId) {
    const PROMISE_YIELD_REGISTER_ID: u64 = 0;

    let promise_idx = env::promise_yield_create(
        function_name,
        arguments,
        gas,
        weight,
        PROMISE_YIELD_REGISTER_ID,
    );

    let mut yield_id = [0; size_of::<YieldId>()];
    unsafe {
        sys::read_register(PROMISE_YIELD_REGISTER_ID, yield_id.as_mut_ptr() as u64);
    };

    (promise_idx, yield_id)
}
