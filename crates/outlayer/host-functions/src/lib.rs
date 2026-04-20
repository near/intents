wasmtime::component::bindgen!({
    path: "../wit",
    world: "imports",
});

pub use self::outlayer::*;

pub trait Host: outlayer::crypto::ed25519::Host + outlayer::crypto::secp256k1::Host + Send {}

impl<T> Host for T where
    T: outlayer::crypto::ed25519::Host + outlayer::crypto::secp256k1::Host + Send
{
}
