// #[cfg(feature = "ed25519")]
// pub mod ed25519;
// // #[cfg(feature = "secp256k1")]
// pub mod secp256k1;

// #[cfg(target_family = "wasm")]
pub mod sys;

// #[cfg(target_family = "wasm")]
pub type Host = sys::SysHost;
// #[cfg(not(target_family = "wasm"))]
// pub type Host = defuse_outlayer_host::DefaultHost;
