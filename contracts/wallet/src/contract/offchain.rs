use defuse_nep641::{OffchainMessage, PendingAuthorization, Proof, contract::AuthResolver};
use near_sdk::{FunctionError, env};

use crate::{
    OffchainSignatureSchema, WalletOffchainProof,
    contract::{Error, Result, WalletImpl},
};

impl<S> AuthResolver for WalletImpl<S>
where
    S: OffchainSignatureSchema,
{
    #[inline]
    fn w_resolve_auth(&self, msg: OffchainMessage, proof: Proof) -> Vec<PendingAuthorization> {
        self.resolve_auth(msg, proof)
            .unwrap_or_else(|err| err.panic())
    }
}

impl<S> WalletImpl<S>
where
    S: OffchainSignatureSchema,
{
    fn resolve_auth(
        &self,
        msg: OffchainMessage,
        proof: Proof,
    ) -> Result<Vec<PendingAuthorization>> {
        // check chain_id
        if msg.chain_id != env::chain_id() {
            return Err(Error::InvalidChainId);
        }

        // check resolver_id
        if msg.resolver_id != env::current_account_id() {
            // TODO: error InvalidResolverId
            return Err(Error::InvalidSignerId(msg.signer_id));
        }

        let WalletOffchainProof {
            as_extension_id,
            proof,
        } = serde_json::from_str(&proof)?;

        if let Some(extension_id) = as_extension_id {
            // check whether extension is enabled
            self.check_extension_enabled(&extension_id)?;

            // forward `w_resolve_auth()` to the extension
            return Ok(vec![PendingAuthorization {
                resolver_id: extension_id,
                proof,
            }]);
        }

        if !self.0.is_signature_allowed() {
            return Err(Error::SignatureDisabled);
        }

        if !S::verify_offchain_msg(&self.0.public_key, &msg, &proof) {
            return Err(Error::InvalidSignature);
        }

        // signature is valid, terminate our resolve branch
        Ok(vec![])
    }
}
