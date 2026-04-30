pub mod crypto;

use std::borrow::Cow;

pub use defuse_outlayer_crypto::signer::InMemorySigner;
pub use defuse_outlayer_primitives as primitives;
use defuse_outlayer_primitives::AppId;

use crate::crypto::CryptoHost;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppContext<'a> {
    pub app_id: AppId<'a>,
}

pub struct State<'a> {
    ctx: AppContext<'a>,
    signer: Cow<'a, InMemorySigner>,
}

impl<'a> State<'a> {
    pub const fn new(ctx: AppContext<'a>, signer: Cow<'a, InMemorySigner>) -> Self {
        Self { ctx, signer }
    }
}

/// Trait defining the host functions available to the component
pub trait HostFunctions: Send + CryptoHost {}

impl<T: Send + CryptoHost> HostFunctions for T {}
