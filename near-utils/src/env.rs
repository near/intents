use near_sdk::{PromiseError, env, sys};

/// Register used internally for atomic operations. This register is safe to use by the user,
/// since it only needs to be untouched while methods of `Environment` execute, which is guaranteed
/// guest code is not parallel.
const ATOMIC_OP_REGISTER: u64 = u64::MAX - 2;

const REGISTER_EXPECTED_ERR: &str =
    "Register was expected to have data because we just wrote it into it.";

#[inline]
#[track_caller]
fn expect_register<T>(option: Option<T>) -> T {
    option.unwrap_or_else(|| env::panic_str(REGISTER_EXPECTED_ERR))
}

/// Same as [`read_register`] but bounded: if data in the register is
/// longer than `max_len`, then `Some(Err(len))` is returned.
pub fn read_register_bounded(register_id: u64, max_len: usize) -> Option<Result<Vec<u8>, usize>> {
    // Get register length and convert to a usize. The max register size in config is much less
    // than the u32 max so the abort should never be hit, but is there for safety because there
    // would be undefined behaviour during `read_register` if the buffer length is truncated.
    let len: usize = env::register_len(register_id)?
        .try_into()
        .unwrap_or_else(|_| env::abort());

    if len > max_len {
        return Some(Err(len));
    }

    // Initialize buffer with capacity.
    let mut buffer = Vec::with_capacity(len);

    // Read register into buffer.
    //* SAFETY: This is safe because the buffer is initialized with the exact capacity of the
    //*         register that is being read from.
    #[allow(clippy::as_conversions)]
    unsafe {
        sys::read_register(register_id, buffer.as_mut_ptr() as u64);

        // Set updated length after writing to buffer.
        buffer.set_len(len);
    }
    Some(Ok(buffer))
}

pub fn promise_result_checked(result_idx: u64, max_len: usize) -> Result<Vec<u8>, PromiseError> {
    promise_result_internal(result_idx)?;
    expect_register(read_register_bounded(ATOMIC_OP_REGISTER, max_len))
        // near-sdk 5.24 uses PromiseError::TooLong(usize), but this type is not yet in near-sdk we currently use
        .map_err(|_| PromiseError::Failed)
}

pub(crate) fn promise_result_internal(result_idx: u64) -> Result<(), PromiseError> {
    match unsafe { sys::promise_result(result_idx, ATOMIC_OP_REGISTER) } {
        1 => Ok(()),
        2 => Err(PromiseError::Failed),
        _ => env::abort(),
    }
}
