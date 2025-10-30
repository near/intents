use defuse_bitmap::{U248, U256};

use defuse_map_utils::Map;
use near_sdk::{
    near,
    store::{LookupMap, key::Sha256},
};

use defuse_core::{DefuseError, Nonce, NoncePrefix, Nonces, Result};

pub type MaybeLegacyAccountNonces =
    MaybeLegacyNonces<LookupMap<U248, U256, Sha256>, LookupMap<U248, U256>>;

#[derive(Debug, Default, Clone)]
#[near(serializers = [borsh])]
pub struct MaybeLegacyNonces<T, L>
where
    T: Map<K = U248, V = U256>,
    L: Map<K = U248, V = U256>,
{
    nonces: Nonces<T>,
    legacy: Option<Nonces<L>>,
}

impl<T, L> MaybeLegacyNonces<T, L>
where
    T: Map<K = U248, V = U256>,
    L: Map<K = U248, V = U256>,
{
    #[inline]
    pub const fn new(nonces: T) -> Self {
        Self {
            //  NOTE: new nonces should not have an legacy part - this is a more efficient use of storage
            legacy: None,
            nonces: Nonces::new(nonces),
        }
    }

    #[inline]
    pub const fn with_legacy(legacy: Nonces<L>, nonces: T) -> Self {
        Self {
            legacy: Some(legacy),
            nonces: Nonces::new(nonces),
        }
    }

    #[inline]
    pub fn commit(&mut self, nonce: Nonce) -> Result<()> {
        // Check legacy maps for used nonce
        if self
            .legacy
            .as_ref()
            .is_some_and(|legacy| legacy.is_used(nonce))
        {
            return Err(DefuseError::NonceUsed);
        }

        // New nonces can be committed only to the new map
        self.nonces.commit(nonce)
    }

    #[inline]
    pub fn is_used(&self, nonce: Nonce) -> bool {
        self.nonces.is_used(nonce)
            || self
                .legacy
                .as_ref()
                .is_some_and(|legacy| legacy.is_used(nonce))
    }

    #[inline]
    pub fn cleanup_by_prefix(&mut self, prefix: NoncePrefix) -> bool {
        self.nonces.cleanup_by_prefix(prefix)
    }
}

#[cfg(test)]
pub(super) mod tests {

    use super::*;
    use near_sdk::{test_utils::VMContextBuilder, testing_env};
    use proptest::{collection::vec, prelude::*};

    fn get_legacy_map(nonces: &[U256], prefix: Vec<u8>) -> Nonces<LookupMap<U248, U256>> {
        let mut legacy_nonces = Nonces::new(LookupMap::new(prefix));
        for nonce in nonces {
            legacy_nonces
                .commit(*nonce)
                .expect("unable to commit nonce");
        }

        legacy_nonces
    }

    fn increase_max_gas() {
        let context = VMContextBuilder::new().build();
        testing_env!(context);
    }

    fn nonces() -> impl Strategy<Value = Vec<U256>> {
        vec(any::<U256>(), 10..100)
    }

    fn storage_prefixes() -> impl Strategy<Value = Vec<u8>> {
        vec(any::<u8>(), 50..=1000)
    }

    proptest! {
        #[test]
        fn new_from_legacy(nonces in nonces(), storage_prefix in storage_prefixes()) {
            increase_max_gas();

            let legacy_nonces = get_legacy_map(&nonces, storage_prefix.clone());
            let new = MaybeLegacyAccountNonces::with_legacy(
                legacy_nonces,
                LookupMap::with_hasher(storage_prefix),
            );

            let legacy_map = new.legacy.as_ref().expect("No legacy nonces present");

            for nonce in &nonces {
                assert!(legacy_map.is_used(*nonce));
                assert!(!new.nonces.is_used(*nonce));
                assert!(new.is_used(*nonce));
            }
        }
    }

    proptest! {
        #[test]
        #[allow(clippy::tuple_array_conversions)]
        fn commit_new_nonce(storage_prefix in storage_prefixes(), new_nonce: [u8;32], legacy_nonce: [u8; 32]) {
            increase_max_gas();
            prop_assume!(new_nonce != legacy_nonce);
            let mut new = MaybeLegacyAccountNonces::new(LookupMap::with_hasher(storage_prefix));

            new.commit(new_nonce)
                .expect("should be able to commit new nonce");
            new.commit(legacy_nonce)
                .expect("should be able to commit new legacy nonce");

            assert!(new.legacy.is_none());

            for &n in &[new_nonce, legacy_nonce] {
                assert!(new.nonces.is_used(n));
                assert!(new.is_used(n));
            }
        }
    }

    proptest! {
        #[test]
        fn commit_existing_legacy_nonce(nonces in nonces(), storage_prefix in storage_prefixes()) {
            increase_max_gas();
            let legacy_nonces = get_legacy_map(&nonces, storage_prefix.clone());
            let mut new = MaybeLegacyAccountNonces::with_legacy(
                legacy_nonces,
                LookupMap::with_hasher(storage_prefix),
            );

            for nonce in &nonces {
            assert!(matches!(
                new.commit(*nonce),
                Err(DefuseError::NonceUsed)
            ));
            }
        }
    }

    proptest! {
        #[test]
        fn commit_duplicate_nonce(nonce: U256, storage_prefix in storage_prefixes()) {
            let mut new = MaybeLegacyAccountNonces::new(LookupMap::with_hasher(storage_prefix));
            new.commit(nonce).expect("First commit should succeed");

            assert!(matches!(
                new.commit(nonce),
                Err(DefuseError::NonceUsed)
            ));
        }
    }

    proptest! {
        #[test]
        fn check_used_nonces(legacy_nonces in nonces(), random_nonces in nonces(), storage_prefix in storage_prefixes()) {
            increase_max_gas();
            let legacy_map = get_legacy_map(&legacy_nonces, storage_prefix.clone());
            let mut new =
                MaybeLegacyAccountNonces::with_legacy(legacy_map, LookupMap::with_hasher(storage_prefix));

            for nonce in &random_nonces {
                new.commit(*nonce).expect("unable to commit nonce");
            }

            for nonce in random_nonces.iter().chain(&legacy_nonces) {
                assert!(new.is_used(*nonce));
            }
        }
    }

    proptest! {
        #[test]
        fn legacy_nonces_cant_be_cleared(storage_prefix in storage_prefixes(), random_nonce : U256) {
            let legacy_nonces = get_legacy_map(&[random_nonce], storage_prefix.clone());
            let mut new = MaybeLegacyAccountNonces::with_legacy(
                legacy_nonces,
                LookupMap::with_hasher(storage_prefix),
            );

            let [prefix @ .., _] = random_nonce;
            assert!(!new.cleanup_by_prefix(prefix));
            assert!(new.is_used(random_nonce));
        }
    }
}
