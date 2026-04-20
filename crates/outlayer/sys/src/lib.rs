wit_bindgen::generate!({
    path: "../wit",
    world: "imports",
    generate_all,
});

pub use self::outlayer::*;
