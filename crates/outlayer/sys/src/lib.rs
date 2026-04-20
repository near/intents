#[cfg(feature = "guest")]
pub mod guest {
    wit_bindgen::generate!({
        path: "../wit",
        world: "imports",
        generate_all,
    });

    pub use self::outlayer::*;
}

#[cfg(feature = "host")]
pub mod host {
    wasmtime::component::bindgen!({
        path: "../wit",
        world: "imports",
    });

    pub use self::outlayer::*;

    pub trait Host:
        outlayer::crypto::ed25519::Host + outlayer::crypto::secp256k1::Host + Send
    {
    }

    impl<T> Host for T where
        T: outlayer::crypto::ed25519::Host + outlayer::crypto::secp256k1::Host + Send
    {
    }
}
