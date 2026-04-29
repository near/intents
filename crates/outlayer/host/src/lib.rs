pub mod bindings;
pub mod crypto;

use std::borrow::Cow;

pub use defuse_outlayer_crypto::signer::InMemorySigner;
pub use defuse_outlayer_primitives as primitives;
use defuse_outlayer_primitives::AppId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context<'a> {
    pub app_id: AppId<'a>,
}

#[derive(Clone)]
pub struct State<'a> {
    ctx: Context<'a>,
    signer: Cow<'a, InMemorySigner>,
}

impl<'a> State<'a> {
    pub const fn new(ctx: Context<'a>, signer: Cow<'a, InMemorySigner>) -> Self {
        Self { ctx, signer }
    }
}

pub trait HostFunctions:
    bindings::outlayer::crypto::ed25519::Host + bindings::outlayer::crypto::secp256k1::Host + Send
{
}

impl<T> HostFunctions for T where
    T: bindings::outlayer::crypto::ed25519::Host
        + bindings::outlayer::crypto::secp256k1::Host
        + Send
{
}
