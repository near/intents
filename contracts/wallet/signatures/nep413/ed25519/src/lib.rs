use defuse_wallet::wallet;
use defuse_wallet_nep413::{WalletNep413, ed25519::Ed25519};

wallet! {
    #[wallet(
        schema = WalletNep413<Ed25519>,
        metadata(
            standard(standard = "wallet-nep413-ed25519", version = "1.0.0")
        )
    )]
    struct Contract(_);
}
