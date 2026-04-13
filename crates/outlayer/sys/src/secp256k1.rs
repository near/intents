wit_bindgen::generate!({
    path: "../wit/crypto.wit",
    world: "secp256k1-world",
});

pub use outlayer::host::secp256k1;
