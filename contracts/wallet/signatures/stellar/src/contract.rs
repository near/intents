use defuse_wallet::wallet;

use crate::WalletStellar;

wallet! {
    #[wallet(
        schema = WalletStellar,
        metadata(
            standard(standard = "wallet-stellar", version = "1.0.0")
        )
    )]
    struct Contract(_);
}
