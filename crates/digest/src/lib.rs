//! Helper crate to automatically chose a backend for digest implementations.
//! Currently supported backends are:
//!
//! * `cfg(near)`: via Near host-function
//! * default: fallback to pure Rust implementation

pub use digest::*;

#[cfg(feature = "ripemd")]
pub mod ripemd;
#[cfg(feature = "sha2")]
pub mod sha2;
#[cfg(feature = "sha3")]
pub mod sha3;
#[cfg(near)]
mod utils;

macro_rules! digest_cfg {
    ($vis:vis struct $name:ident { $($tt:tt)* }) => {
        #[derive(Debug, Clone, Default)]
        #[repr(transparent)]
        $vis struct $name(cfg_select! {$($tt)*});

        impl ::digest::OutputSizeUser for $name {
            type OutputSize = <cfg_select! {$($tt)*} as ::digest::OutputSizeUser>::OutputSize;
        }

        impl ::digest::Update for $name {
            #[inline]
            fn update(&mut self, data: &[u8]) {
                ::digest::Update::update(&mut self.0, data);
            }
        }

        impl ::digest::FixedOutput for $name {
            #[inline]
            fn finalize_into(self, out: &mut ::digest::Output<Self>) {
                ::digest::FixedOutput::finalize_into(self.0, out);
            }
        }

        impl ::digest::HashMarker for $name {}
    };
}
pub(crate) use digest_cfg;
