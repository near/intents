#[cfg(feature = "digest")]
pub mod digest;
mod event;
pub use event::{NearSdkLog, REFUND_MEMO, TOTAL_LOG_LENGTH_LIMIT};
mod gas;
mod lock;
mod panic;
mod panic_on_clone;
mod prefix;
pub mod promise;
#[cfg(feature = "time")]
pub mod time;

pub use self::{
    gas::*, lock::*, panic::*, panic_on_clone::*, prefix::*,
    promise::{PromiseJsonResult, promise_result_checked_json, promise_result_checked_json_with_args, promise_result_checked_void},
};

#[macro_export]
macro_rules! method_name {
    ($ty:ident::$method:ident) => {{
        // check that method exists
        const _: *const () = $ty::$method as *const ();
        stringify!($method)
    }};
}
