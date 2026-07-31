mod event;
pub use event::{REFUND_MEMO, TOTAL_LOG_LENGTH_LIMIT};
mod panic_on_clone;

#[cfg(feature = "near-contract")]
mod promise;

pub use self::panic_on_clone::*;
#[cfg(feature = "near-contract")]
pub use self::promise::*;

#[macro_export]
macro_rules! method_name {
    ($ty:ident::$method:ident) => {{
        // check that method exists
        const _: *const () = $ty::$method as *const ();
        stringify!($method)
    }};
}
