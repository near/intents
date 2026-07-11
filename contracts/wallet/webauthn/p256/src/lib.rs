use defuse_wallet::wallet;
use defuse_wallet_webauthn::{WalletWebauthn, p256::P256, webauthn::IgnoreUserVerification};

wallet! {
    #[wallet(
        schema = WalletWebauthn<
            P256,
            // `UV` (User Verified) flag is only set by FIDO2-capable devices with
            // PIN / biometric setup.
            //
            // FIDO U2F (CTAP 1) authenticators (such as old Ledger and Yubikey
            // devices) only set `UP` (User Present) flag and doesn't support `UV`
            // (User Verified).
            IgnoreUserVerification,
        >,
        metadata(
            standard(standard = "wallet-webauthn-p256", version = "1.0.0")
        )
    )]
    struct Contract(_);
}
