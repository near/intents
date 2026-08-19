use defuse_wallet::wallet;

use crate::WalletSr25519;

wallet! {
    #[wallet(
        schema = WalletSr25519,
        metadata(
            standard(standard = "wallet-sr25519", version = "1.0.0")
        )
    )]
    struct Contract(_);
}
