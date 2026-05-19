pub mod ed25519;
pub mod secp256k1;

#[cfg(any(feature = "kdf", test))]
pub use defuse_kdf::{self as kdf, *};

#[cfg(any(feature = "kdf", test))]
#[derive(Debug, Clone, Copy, Default)]
pub struct Signer;

#[cfg(any(feature = "kdf", test))]
#[impl_tools::autoimpl(Debug, Clone, Default)]
#[derive(Copy)]
pub struct HostSchema<C: ::defuse_kdf::Curve>(::core::marker::PhantomData<C>);
