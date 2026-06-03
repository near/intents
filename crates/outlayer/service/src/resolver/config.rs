use std::time::Duration;

use url::Url;

use crate::resolver::{HttpResolver, NearResolver, Resolver, UrlResolver};

const NEAR_RPC_URL: &str = "https://rpc.mainnet.near.org";
const NEAR_CHAIN_ID: &str = "mainnet";
const MAX_WASM_SIZE_10MB: usize = 10 * 1024 * 1024;
const NEAR_CACHE_TTL: Duration = Duration::from_millis(200);

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Deserialize),
    serde(deny_unknown_fields, default)
)]
pub struct ResolverConfig {
    pub near_rpc_url: Url,
    pub near_chain_id: String,
    pub http_max_body_bytes: usize,
    #[cfg_attr(
        feature = "serde",
        serde(rename = "near_cache_ttl_milliseconds"),
        serde_as(as = "::serde_with::DurationMilliSeconds<u64>")
    )]
    pub near_cache_ttl: Duration,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            near_rpc_url: NEAR_RPC_URL.parse().expect("valid mainnet RPC URL"),
            near_chain_id: NEAR_CHAIN_ID.to_owned(),
            http_max_body_bytes: MAX_WASM_SIZE_10MB,
            near_cache_ttl: NEAR_CACHE_TTL,
        }
    }
}

impl ResolverConfig {
    pub fn build(self) -> Resolver {
        let near = near_kit::Near::custom(self.near_rpc_url, self.near_chain_id).build();
        let near = NearResolver::new(near, self.near_cache_ttl);
        let url = UrlResolver::new(HttpResolver::new(self.http_max_body_bytes));
        Resolver::new(near, url)
    }
}
