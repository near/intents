use defuse_wallet_ed25519::{WalletEd25519, WalletEd25519Signer, crypto::ed25519::ed25519_dalek};
use defuse_wallet_sdk::{
    AccountIdRef, Wallet,
    mpc::kdf::{
        DeriveSigner, RecoverableDeriveSigner,
        crypto::{
            RecoverableCurve,
            secp256k1::{Secp256k1, Secp256k1RecoverableSignature, Secp256k1UncompressedPublicKey},
        },
    },
};
use hex_literal::hex;
use near_kit::Near;
use rand::{rand_core::UnwrapErr, rngs::SysRng};

const WALLET_GLOBAL_CONTRACT_ID: &AccountIdRef =
    AccountIdRef::new_or_panic("0sb0d7ef4f935c6ef78e08ad03569767aaec4223a3");

const PATH: &str = "derivation path";

#[tokio::main]
async fn main() {
    let near = Near::from_env().unwrap();

    // 1) Generate a keypair and Build a wallet
    let signer = ed25519_dalek::SigningKey::generate(&mut UnwrapErr(SysRng));
    let wallet = Wallet::<WalletEd25519>::new(
        WALLET_GLOBAL_CONTRACT_ID.to_owned(),
        WalletEd25519Signer(signer),
    )
    .with_client(near.clone())
    .with_relayer(near);

    println!("wallet account ID: {}", wallet.account_id());

    // 1) Prepare MPC signer
    let mpc_signer = wallet.mpc_signer::<Secp256k1>(0).await.unwrap();

    // 2) Derive MPC public key
    let derived_pk = mpc_signer.derive_public_key(PATH);
    println!(
        "derived public key: {}",
        Secp256k1UncompressedPublicKey::from(derived_pk)
    );

    // 3) Sign via MPC
    let prehash = hex!("0128fdba02691843069aba70c0523b9c43f4b0de4e34962462b0525490780a53");
    let started_at = tokio::time::Instant::now();
    let (signature, recovery_id) = mpc_signer
        // secp256k1 needs 32 byte prehash
        .derive_sign_recoverable(PATH, &prehash)
        .await
        .unwrap();
    println!(
        "signature: {}, (elapsed: {:?})",
        Secp256k1RecoverableSignature::from((signature, recovery_id)),
        started_at.elapsed(),
    );

    // 4) Verify the signature
    assert_eq!(
        Secp256k1::recover(&prehash, &signature, recovery_id),
        Some(derived_pk)
    );
}
