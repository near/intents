use defuse_wallet::wallet;
use defuse_wallet_webauthn::{WalletWebauthn, p256::P256, webauthn::IgnoreUserVerification};

wallet! {
    #[wallet(
        schema = WalletWebauthn<P256, IgnoreUserVerification>,
        metadata(
            standard(standard = "wallet-webauthn-p256", version = "1.0.0")
        )
    )]
    // TODO: ignore user verification?
    struct Contract(_);
}
