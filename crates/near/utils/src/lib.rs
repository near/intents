mod event;
pub use event::{REFUND_MEMO, TOTAL_LOG_LENGTH_LIMIT};
mod promise;

#[cfg(feature = "borsh")]
mod panic_on_clone;

#[cfg(feature = "borsh")]
pub use self::panic_on_clone::*;

pub use self::promise::*;

#[macro_export]
macro_rules! method_name {
    ($ty:ident::$method:ident) => {{
        // check that method exists
        const _: *const () = $ty::$method as *const ();
        stringify!($method)
    }};
}
