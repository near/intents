use defuse_crypto::{RecoverableSigner, secp256k1::Secp256k1};
use defuse_wallet::offchain::OffchainMessage;
use defuse_wallet_sdk::{Proof, RequestMessage, WalletSigner};

use crate::{
    Eip712AuthMessage, Eip712Message, Eip712RequestMessage, EthAddress, SignedEip712, WalletEip712,
};

/// Signer wrapper for [`WalletEip712`] signature schema.
///
/// It builds the same typed data an Ethereum wallet would be asked to sign
/// via `eth_signTypedData_v4` and signs its
/// [prehash](Eip712Message::prehash).
///
/// # Examples
///
/// ```rust
/// use defuse_wallet_eip712::{WalletEip712, WalletEip712Signer, crypto::secp256k1::k256};
/// use defuse_wallet_sdk::{Request, SignatureSchema, Wallet};
/// # use defuse_wallet_sdk::GlobalContractId;
/// # use hex_literal::hex;
/// use k256::elliptic_curve::Generate;
/// # const GLOBAL_CONTRACT_ID: GlobalContractId = GlobalContractId::CodeHash(
/// #     hex!("0000000000000000000000000000000000000000000000000000000000000000"),
/// # );
///
/// # tokio_test::block_on(async {
/// let signer = k256::ecdsa::SigningKey::generate();
/// let wallet = Wallet::<WalletEip712>::new(
///     GLOBAL_CONTRACT_ID,
///     WalletEip712Signer(signer),
/// );
///
/// let (msg, proof) = wallet.sign(Request::new()).await?;
///
/// assert!(
///     WalletEip712::verify_request_msg(&wallet.public_key(), &msg, &proof),
///     "signer produced invalid signature",
/// );
/// # Ok::<_, Box<dyn core::error::Error>>(()) }).unwrap();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::From, derive_more::AsRef)]
pub struct WalletEip712Signer<S>(pub S);

impl<S> WalletEip712Signer<S>
where
    S: RecoverableSigner<Secp256k1>,
{
    async fn sign_typed_data<M>(&self, message: M) -> Result<Proof, S::Error>
    where
        M: Eip712Message + ::serde::Serialize,
    {
        let signature = self.0.sign_recoverable(&message.prehash()).await?;

        Ok(serde_json::to_string(&SignedEip712 {
            message,
            signature: signature.into(),
        })
        .unwrap_or_else(|_| unreachable!()))
    }
}

impl<S> WalletSigner<WalletEip712> for WalletEip712Signer<S>
where
    S: RecoverableSigner<Secp256k1>,
{
    type Error = S::Error;

    #[inline]
    fn public_key(&self) -> EthAddress {
        EthAddress::from(&self.0.public_key())
    }

    async fn sign_request_msg(&self, msg: &RequestMessage) -> Result<Proof, Self::Error> {
        self.sign_typed_data(Eip712RequestMessage::from(msg)).await
    }

    async fn sign_offchain_msg(&self, msg: &OffchainMessage) -> Result<Proof, Self::Error> {
        self.sign_typed_data(Eip712AuthMessage::from(msg)).await
    }
}
