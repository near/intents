pub mod nep141;
mod nep171;
mod nep245;

// TODO: make it prettier
pub const MT_RECEIVER_STUB_WASM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../res/multi-token-receiver-stub/multi_token_receiver_stub.wasm"
));
