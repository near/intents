wit_bindgen::generate!({
    path: "../wit/secp256k1.wit",
    world: "imports",
});

pub use outlayer::host::secp256k1;
