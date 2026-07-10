use defuse_wallet::wallet;
use defuse_wallet_webauthn::{WalletWebauthn, ed25519::Ed25519, webauthn::IgnoreUserVerification};

// TODO
// #[cfg_attr(not(near), allow(dead_code))]
wallet! {
    #[wallet(
        schema = WalletWebauthn<Ed25519, IgnoreUserVerification>,
        metadata(
            standard(standard = "wallet-webauthn-ed25519", version = "1.0.0")
        )
    )]
    // TODO: ignore user verification?
    struct Contract(_);
}
