#[cfg(feature = "ed25519")]
pub mod ed25519;
#[cfg(feature = "secp256k1")]
pub mod secp256k1;

pub struct SysHost;

#[cfg(target_family = "wasm")]
pub type Host = SysHost;
#[cfg(not(target_family = "wasm"))]
pub type Host = defuse_outlayer_host::DefaultHost;
