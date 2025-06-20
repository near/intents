#[cfg(feature = "arbitrary")]
pub mod arbitrary;
mod cache;
#[cfg(feature = "digest")]
pub mod digest;
mod gas;
mod lock;
mod panic;
mod panic_on_clone;
mod prefix;
#[cfg(feature = "time")]
pub mod time;

pub use self::{cache::*, gas::*, lock::*, panic::*, panic_on_clone::*, prefix::*};

#[macro_export]
macro_rules! method_name {
    ($ty:ident::$method:ident) => {{
        // check that method exists
        const _: *const () = $ty::$method as *const ();
        stringify!($method)
    }};
}
