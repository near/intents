use defuse_bitmap::{U248, U256};

use defuse_near_utils::NestPrefix;
use near_sdk::{
    BorshStorageKey, IntoStorageKey,
    borsh::BorshSerialize,
    near,
    store::{LookupMap, key::Sha256},
};

use defuse_core::{DefuseError, ExpirableNonce, Nonces, Result};

#[derive(Debug)]
#[near(serializers = [borsh])]
pub struct MaybeOptimizedNonces {
    legacy: Option<Nonces<LookupMap<U248, U256>>>,
    nonces: Nonces<LookupMap<U248, U256, Sha256>>,
}

impl MaybeOptimizedNonces {
    pub fn new<S>(prefix: S) -> Self
    where
        S: IntoStorageKey,
    {
        let prefix = prefix.into_storage_key();

        Self {
            //  NOTE: new nonces should not have an legacy part - this is a more efficient use of storage
            legacy: None,
            nonces: Nonces::new(LookupMap::with_hasher(
                prefix.as_slice().nest(NoncePrefix::Nonces),
            )),
        }
    }

    pub fn new_with_legacy<S>(prefix: S, legacy: Nonces<LookupMap<U248, U256>>) -> Self
    where
        S: IntoStorageKey,
    {
        let prefix = prefix.into_storage_key();

        Self {
            legacy: Some(legacy),
            nonces: Nonces::new(LookupMap::with_hasher(
                prefix.as_slice().nest(NoncePrefix::Nonces),
            )),
        }
    }

    #[inline]
    pub fn commit_nonce(&mut self, nonce: U256) -> Result<()> {
        if ExpirableNonce::maybe_from(nonce).is_some_and(|expirable| expirable.has_expired()) {
            return Err(DefuseError::NonceExpired);
        }

        // Check both maps for used nonce
        if self.is_nonce_used(nonce) {
            return Err(DefuseError::NonceUsed);
        }

        // New nonces can be committed only to the new map
        self.nonces
            .commit(nonce)
            .then_some(())
            .ok_or(DefuseError::NonceUsed)?;

        Ok(())
    }

    #[inline]
    pub fn is_nonce_used(&self, nonce: U256) -> bool {
        // Check legacy map only if the nonce is not expirable
        // otherwise check both maps
        self.nonces.is_used(nonce)
            || (ExpirableNonce::maybe_from(nonce).is_none()
                && self
                    .legacy
                    .as_ref()
                    .is_some_and(|legacy| legacy.is_used(nonce)))
    }

    #[inline]
    pub fn clear_expired_nonce(&mut self, nonce: U256) -> bool {
        // Expirable nonces can not be in the legacy map
        ExpirableNonce::maybe_from(nonce).is_some_and(|n| n.has_expired())
            && self.nonces.clear_expired(nonce)
    }
}

// TODO: check on collisions
#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "::near_sdk::borsh")]
enum NoncePrefix {
    _Legacy, // should be the same as AccountPrefix::Nonces
    Nonces,
}
