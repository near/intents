wasmtime::component::bindgen!({
    path: "../wit",
    world: "imports",
    imports: {
        default: trappable | tracing,
    },
    ownership: Borrowing {
        duplicate_if_necessary: true
    },
});
