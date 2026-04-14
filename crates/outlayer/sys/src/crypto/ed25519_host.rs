wit_bindgen::generate!({
    path: "../wit/ed25519.wit",
    world: "imports",
});

pub use outlayer::host::ed25519;
