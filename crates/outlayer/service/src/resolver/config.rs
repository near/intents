use crate::resolver::{HttpResolver, NearResolver, Resolver, UrlResolver};

const NEAR_RPC_URL: &str = "https://rpc.mainnet.near.org";
const MAX_WASM_SIZE_10MB: usize = 10 * 1024 * 1024;

#[cfg_attr(
    feature = "serde",
    ::cfg_eval::cfg_eval,
    ::serde_with::serde_as,
    derive(::serde::Serialize, ::serde::Deserialize),
    serde(deny_unknown_fields, default)
)]
pub struct ResolverConfig {
    pub near_rpc_url: String,
    #[cfg_attr(feature = "serde", serde_as(as = "::serde_with::DisplayFromStr"))]
    pub near_chain_id: near_kit::ChainId,
    pub http_max_len: usize,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            near_rpc_url: NEAR_RPC_URL.to_string(),
            near_chain_id: near_kit::ChainId::mainnet(),
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

    pub fn build(self) -> Resolver {
        let near = near_kit::Near::custom(self.0.near_rpc_url, self.0.near_chain_id).build();
        let near = NearResolver::new(near);
        let url = UrlResolver::new(HttpResolver::new(self.0.http_max_len));
        Resolver::new(near, url)
    }
}
