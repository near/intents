use crate::resolver::{NearResolver, Resolver, UrlResolver, HttpResolver};

const NEAR_RPC_URL: &str = "https://rpc.mainnet.near.org";
const NEAR_CHAIN_ID: &str = "mainnet";
const MAX_WASM_SIZE_10MB: usize = 10 * 1024 * 1024;


#[cfg_attr(
    feature = "serde",
    derive(::serde::Serialize, ::serde::Deserialize),
    serde(deny_unknown_fields, default)
)]
pub struct ResolverConfig {
    pub near_rpc_url: String,
    pub near_chain_id: String,
    pub http_max_len: usize,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            near_rpc_url: NEAR_RPC_URL.to_string(),
            near_chain_id: NEAR_CHAIN_ID.to_string(),
            http_max_len: MAX_WASM_SIZE_10MB,
        }
    }
}

#[derive(Default)]
pub struct ResolverBuilder(ResolverConfig);

impl ResolverBuilder {
    #[must_use]
    pub fn with_config(mut self, config: ResolverConfig) -> Self {
        self.0 = config;
        self
    }

    pub fn build(config: ResolverConfig) -> Resolver {
        let near = near_kit::Near::custom(config.near_rpc_url, config.near_chain_id).build();
        let near = NearResolver::new(near);
        let url = UrlResolver::new(HttpResolver::new(config.http_max_len));
        Resolver::new(near, url)
    }

}
