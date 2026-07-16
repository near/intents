use defuse_crypto::{
    Signer,
    ed25519::{Ed25519, Ed25519PublicKey, Ed25519Signature},
};
use defuse_wallet_sdk::{Proof, RequestMessage, WalletSigner};

use crate::WalletEd25519;

/// Signer wrapper for [`WalletEd25519`] signature schema.
///
/// # Examples
///
/// ```rust
/// use defuse_wallet_ed25519::{WalletEd25519, WalletEd25519Signer};
/// use defuse_wallet_sdk::{Request, SignatureSchema, Wallet};
/// # use defuse_wallet_sdk::GlobalContractId;
/// # use hex_literal::hex;
/// use rand::{rand_core::UnwrapErr, rngs::SysRng};
/// # const GLOBAL_CONTRACT_ID: GlobalContractId = GlobalContractId::CodeHash(
/// #     hex!("0000000000000000000000000000000000000000000000000000000000000000"),
/// # );
///
/// # tokio_test::block_on(async {
/// let signer = ed25519_dalek::SigningKey::generate(&mut UnwrapErr(SysRng));
/// let wallet = Wallet::<WalletEd25519, _>::new(
///     GLOBAL_CONTRACT_ID,
///     WalletEd25519Signer(signer),
/// );
///
/// let (msg, proof) = wallet.sign(Request::new()).await?;
///
/// assert!(
///     WalletEd25519::verify(&wallet.public_key(), &msg, &proof),
///     "signer produced invalid signature",
/// );
/// # Ok::<_, Box<dyn core::error::Error>>(()) }).unwrap();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::From, derive_more::AsRef)]
pub struct WalletEd25519Signer<S>(pub S);

impl<S> WalletSigner<WalletEd25519> for WalletEd25519Signer<S>
where
    S: Signer<Ed25519>,
{
    type Error = S::Error;

    #[inline]
    fn public_key(&self) -> Ed25519PublicKey {
        self.0.public_key().into()
    }

    async fn sign_request_msg(&self, msg: &RequestMessage) -> Result<Proof, Self::Error> {
        let sig = self.0.sign(&msg.hash()).await?;

        Ok(Ed25519Signature::from(sig).to_string())
    }
}
