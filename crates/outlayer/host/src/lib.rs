pub use defuse_outlayer_crypto as crypto;

pub mod context;

wasmtime::component::bindgen!({
    path: "../wit",
    world: "imports",
    imports: {
        // default: async | trappable,
    },
    ownership: Borrowing {
        duplicate_if_necessary: true
    },
});
