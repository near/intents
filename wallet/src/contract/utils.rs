// TODO: remove in favor of `env::chain_id()` when NEP-638 lands
pub(crate) fn chain_id() -> String {
    "mainnet".to_string()
}
