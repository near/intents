use defuse_crypto::PublicKey;
use defuse_serde_utils::base64::Base64;
use near_sdk::{AccountIdRef, CryptoHash, near};
use serde_with::serde_as;

use crate::{
    Nonce, Result,
    engine::{Engine, Inspector, State},
};

use super::ExecutableIntent;

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
/// Given an account id, the user can add public keys. The added public keys can sign
/// intents on behalf of these accounts, even to add new ones.
/// Warning: Implicit account ids, by default, have their corresponding public keys added.
/// Meaning: For a leaked private key, whose implicit account id had been used in intents,
/// the user must manually rotate the underlying public key within intents, too.
pub struct AddPublicKey {
    pub public_key: PublicKey,
}

impl ExecutableIntent for AddPublicKey {
    #[inline]
    fn execute_intent<S, I>(
        self,
        signer_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        _intent_hash: CryptoHash,
    ) -> Result<()>
    where
        S: State,
        I: Inspector,
    {
        engine
            .state
            .add_public_key(signer_id.to_owned(), self.public_key)
    }
}

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
/// Remove the public key associated with a given account. See `AddPublicKey`.
pub struct RemovePublicKey {
    pub public_key: PublicKey,
}

impl ExecutableIntent for RemovePublicKey {
    #[inline]
    fn execute_intent<S, I>(
        self,
        signer_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        _intent_hash: CryptoHash,
    ) -> crate::Result<()>
    where
        S: State,
        I: Inspector,
    {
        engine
            .state
            .remove_public_key(signer_id.to_owned(), self.public_key)
    }
}

#[cfg_attr(
    all(feature = "abi", not(target_arch = "wasm32")),
    serde_as(schemars = true)
)]
#[cfg_attr(
    not(all(feature = "abi", not(target_arch = "wasm32"))),
    serde_as(schemars = false)
)]
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
/// Every intent execution requires a nonce. Each account id gets (over time, while using the intents contract) more nonces,
/// and this ensures that nonces are not reused to avoid replay attacks. This "marks" the nonce as used.
pub struct InvalidateNonces {
    #[serde_as(as = "Vec<Base64>")]
    pub nonces: Vec<Nonce>,
}

impl ExecutableIntent for InvalidateNonces {
    #[inline]
    fn execute_intent<S, I>(
        self,
        signer_id: &AccountIdRef,
        engine: &mut Engine<S, I>,
        _intent_hash: CryptoHash,
    ) -> crate::Result<()>
    where
        S: State,
        I: Inspector,
    {
        self.nonces
            .into_iter()
            .try_for_each(|n| engine.state.commit_nonce(signer_id.to_owned(), n))
    }
}
