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

    pub trait Host: Send {
        type Ed25519: self::crypto::ed25519::Host + Send;
        type Secp256k1: self::crypto::secp256k1::Host + Send;

        fn ed25519(&mut self) -> &mut Self::Ed25519;
        fn secp256k1(&mut self) -> &mut Self::Secp256k1;
    }
}
