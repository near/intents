use defuse_crypto::{
    Signer,
    sr25519::{Sr25519, Sr25519PublicKey, Sr25519Signature},
};
use defuse_wallet_sdk::{Proof, RequestMessage, WalletSigner};

use crate::WalletSr25519;

/// Signer wrapper for [`WalletSr25519`] signature schema.
///
/// Wraps the canonical [`RequestMessage::hash`] in `<Bytes>...</Bytes>`
/// (as Polkadot.js Extension, Talisman, and other Substrate wallets do on
/// `signRaw`) before delegating to the underlying [`Signer<Sr25519>`].
///
/// # Examples
///
/// ```rust
/// # use defuse_wallet_sr25519::crypto::sr25519::schnorrkel::Keypair;
/// use defuse_wallet_sr25519::{WalletSr25519, WalletSr25519Signer};
/// use defuse_wallet_sdk::{Request, SignatureSchema, Wallet};
/// # use defuse_wallet_sdk::GlobalContractId;
/// # use hex_literal::hex;
/// # const GLOBAL_CONTRACT_ID: GlobalContractId = GlobalContractId::CodeHash(
/// #     hex!("0000000000000000000000000000000000000000000000000000000000000000"),
/// # );
///
/// # tokio_test::block_on(async {
/// let signer = Keypair::generate();
/// let wallet = Wallet::<WalletSr25519, _>::new(
///     GLOBAL_CONTRACT_ID,
///     WalletSr25519Signer(signer),
/// );
///
/// let (msg, proof) = wallet.sign(Request::new()).await?;
///
/// assert!(
///     WalletSr25519::verify(&wallet.public_key(), &msg, &proof),
///     "signer produced invalid signature",
/// );
/// # Ok::<_, Box<dyn core::error::Error>>(()) }).unwrap();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::From, derive_more::AsRef)]
pub struct WalletSr25519Signer<S>(pub S);

impl<S> WalletSigner<WalletSr25519> for WalletSr25519Signer<S>
where
    S: Signer<Sr25519>,
{
    type Error = S::Error;

    #[inline]
    fn public_key(&self) -> Sr25519PublicKey {
        self.0.public_key().into()
    }

    async fn sign_request_msg(&self, msg: &RequestMessage) -> Result<Proof, Self::Error> {
        let sig = self
            .0
            .sign(&WalletSr25519::signed_message(&msg.hash()))
            .await?;

        Ok(Sr25519Signature::from(sig).to_string())
    }
}
