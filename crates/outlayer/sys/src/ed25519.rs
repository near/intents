wit_bindgen::generate!({
    path: "../wit/crypto.wit",
    world: "ed25519-world",
});

pub use outlayer::host::ed25519;
