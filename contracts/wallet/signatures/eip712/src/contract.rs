use defuse_wallet::wallet;

use crate::WalletEip712;

wallet! {
    #[wallet(
        schema = WalletEip712,
        metadata(
            standard(standard = "wallet-eip712", version = "1.0.0")
        )
    )]
    struct Contract(_);
}
