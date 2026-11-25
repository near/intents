use defuse_wallet::wallet;

use crate::WalletSolana;

wallet! {
    #[wallet(
        schema = WalletSolana,
        metadata(
            standard(standard = "wallet-solana", version = "1.0.0")
        )
    )]
    struct Contract(_);
}
