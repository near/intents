#[cfg(target_family = "wasm")]
compile_error!(
    r#"The `host` crate is only meant to be used in a non-WebAssembly environment.
    If you want to use host functionality in a WebAssembly environment,
    please use the `defuse_outlayer_sdk` crate instead.
"#
);

#[cfg(feature = "ed25519")]
pub mod ed25519;
#[cfg(feature = "secp256k1")]
pub mod secp256k1;

pub struct WorkerHost {
    #[cfg(feature = "ed25519")]
    ed25519: ed25519::WorkerEd25519Host,
    #[cfg(feature = "secp256k1")]
    secp256k1: secp256k1::WorkerSecp256k1Host,
}
