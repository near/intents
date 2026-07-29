mod event;
pub use event::{REFUND_MEMO, TOTAL_LOG_LENGTH_LIMIT};
#[cfg(feature = "near-contract")]
mod gas;
mod lock;
mod panic_on_clone;
#[cfg(feature = "near-contract")]
mod prefix;
#[cfg(feature = "near-contract")]
mod promise;

pub use self::{lock::*, panic_on_clone::*};

#[cfg(feature = "near-contract")]
pub use self::{gas::*, prefix::*, promise::*};

#[macro_export]
macro_rules! method_name {
    ($ty:ident::$method:ident) => {{
        // check that method exists
        const _: *const () = $ty::$method as *const ();
        stringify!($method)
    }};
}
