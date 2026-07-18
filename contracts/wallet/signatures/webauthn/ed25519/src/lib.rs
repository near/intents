use defuse_wallet::wallet;
use defuse_wallet_webauthn::{WalletWebauthn, ed25519::Ed25519, webauthn::RequireUserVerification};

wallet! {
    #[wallet(
        schema = WalletWebauthn<
            Ed25519,
            // Require the `UV` (User Verified) flag: every signature MUST be
            // authorized by a biometric / PIN / screen-lock verification, not
            // mere user presence. This wallet's passkey is the sole key over
            // funds, so on-chain enforcement is required — the client also
            // requests `userVerification: "required"`, but a proof submitted
            // directly to the relayer would bypass that; the contract must not
            // accept a user-presence-only assertion.
            //
            // Trade-off: FIDO U2F (CTAP 1) authenticators (e.g. old Ledger /
            // YubiKey without a PIN) only set `UP` and cannot satisfy this.
            // Platform passkeys (Apple/Google/Windows) always perform UV.
            RequireUserVerification,
        >,
        metadata(
            standard(standard = "wallet-webauthn-ed25519", version = "1.0.0")
        )
    )]
    struct Contract(_);
}
