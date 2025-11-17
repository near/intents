use near_sdk::{CryptoHash, Gas, GasWeight, Promise, PromiseIndex, env, sys};

pub trait PromiseExt: Sized {
    fn and_maybe(self, p: Option<Promise>) -> Promise;
}

impl PromiseExt for Promise {
    #[inline]
    fn and_maybe(self, p: Option<Promise>) -> Promise {
        if let Some(p) = p { self.and(p) } else { self }
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
