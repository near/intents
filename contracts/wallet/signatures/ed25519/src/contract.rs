use defuse_wallet::wallet;

use crate::WalletEd25519;

wallet! {
    #[wallet(
        schema = WalletEd25519,
        metadata(
            standard(standard = "wallet-ed25519", version = "1.0.0")
        )
    )]
    struct Contract(_);
}
