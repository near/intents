mod cache;
mod gas;
mod lock;
mod panic;
mod panic_on_clone;
mod prefix;

pub use self::{cache::*, gas::*, lock::*, panic::*, panic_on_clone::*, prefix::*};

#[macro_export]
macro_rules! method_name {
    ($ty:ident::$method:ident) => {{
        // check that method exists
        const _: *const () = $ty::$method as *const ();
        stringify!($method)
    }};
}
