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
    pub http_max_body_bytes: usize,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            near_rpc_url: NEAR_RPC_URL.to_string(),
            near_chain_id: near_kit::ChainId::mainnet(),
            http_max_body_bytes: MAX_WASM_SIZE_10MB,
        }
    }
}

impl ResolverConfig {
    pub fn build(self) -> Resolver {
        let near = near_kit::Near::custom(self.near_rpc_url, self.near_chain_id).build();
        let near = NearResolver::new(near);
        let url = UrlResolver::new(HttpResolver::new(self.http_max_body_bytes));
        Resolver::new(near, url)
    }
}
