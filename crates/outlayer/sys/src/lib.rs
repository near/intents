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
}
