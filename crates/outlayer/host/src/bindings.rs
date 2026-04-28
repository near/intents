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

pub trait HostFunctions:
    outlayer::crypto::ed25519::Host + outlayer::crypto::secp256k1::Host + Send
{
}

impl<T> HostFunctions for T where
    T: outlayer::crypto::ed25519::Host + outlayer::crypto::secp256k1::Host + Send
{
}
