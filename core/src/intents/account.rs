use defuse_crypto::PublicKey;
use defuse_serde_utils::base64::Base64;
use near_sdk::{AccountIdRef, CryptoHash, near};
use serde_with::serde_as;

use crate::{
    DefuseError, Nonce, Result,
    engine::{Engine, Inspector, State},
};

use super::ExecutableIntent;

/// Add public key to the signer account
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
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
        if !engine
            .state
            .add_public_key(signer_id.to_owned(), self.public_key)
        {
            return Err(DefuseError::PublicKeyExists);
        }
        Ok(())
    }
}

/// Remove public key to the signer account
#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
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
        if !engine
            .state
            .remove_public_key(signer_id.to_owned(), self.public_key)
        {
            return Err(DefuseError::PublicKeyNotExist);
        }
        Ok(())
    }
}

/// Invalidate given nonces TODO: error?
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
        for nonce in self.nonces {
            if !engine.state.commit_nonce(signer_id.to_owned(), nonce) {
                return Err(DefuseError::NonceUsed);
            }
        }
        Ok(())
    }
}
