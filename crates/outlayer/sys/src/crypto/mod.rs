// TODO: may be make symlink?
#[cfg(feature = "ed25519")]
mod ed25519_world {
    wit_bindgen::generate!({
        path: "../wit/crypto.wit",
        world: "ed25519-world",
    });
}

#[cfg(feature = "secp256k1")]
mod secp256k1_world {
    wit_bindgen::generate!({
        path: "../wit/crypto.wit",
        world: "secp256k1-world",
    });
}

#[cfg(feature = "ed25519")]
pub use ed25519_world::outlayer::host::ed25519;
#[cfg(feature = "secp256k1")]
pub use secp256k1_world::outlayer::host::secp256k1;
