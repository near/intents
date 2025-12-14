#[cfg(feature = "digest")]
pub mod digest;
mod event;
pub use event::NearSdkLog;
mod gas;
mod lock;
mod panic;
mod panic_on_clone;
mod prefix;
mod promise;
#[cfg(feature = "time")]
pub mod time;

pub use self::{gas::*, lock::*, panic::*, panic_on_clone::*, prefix::*, promise::*};

#[macro_export]
macro_rules! method_name {
    ($ty:ident::$method:ident) => {{
        // check that method exists
        const _: *const () = $ty::$method as *const ();
        stringify!($method)
    }};
}
