use defuse_bitmap::{U248, U256};

use near_sdk::{
    IntoStorageKey, near,
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
            nonces: Nonces::new(LookupMap::with_hasher(prefix)),
        }
    }

    pub fn new_with_legacy<S>(prefix: S, legacy: Nonces<LookupMap<U248, U256>>) -> Self
    where
        S: IntoStorageKey,
    {
        Self {
            legacy: Some(legacy),
            nonces: Nonces::new(LookupMap::with_hasher(prefix)),
        }
    }

    #[inline]
    pub fn commit_nonce(&mut self, nonce: U256) -> Result<()> {
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
    pub fn is_nonce_used(&self, nonce: U256) -> bool {
        // Check legacy map only if the nonce is not expirable
        // otherwise check both maps

        // TODO: legacy nonces which have expirable prefix can be committed twice, check probability!
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
        self.nonces.clear_expired(nonce)
    }
}

#[cfg(test)]
pub(super) mod tests {

    use super::*;

    use chrono::{Days, Utc};
    use defuse_bitmap::U256;
    use defuse_core::{Deadline, ExpirableNonce};
    use defuse_test_utils::random::{Rng, range_to_random_size, rng};

    use rstest::fixture;
    use std::ops::RangeBounds;

    use defuse_test_utils::random::{make_arbitrary, random_bytes};
    use rstest::rstest;

    fn generate_nonce(expirable: bool, mut rng: impl Rng) -> U256 {
        if expirable {
            let future_deadline = Deadline::new(Utc::now().checked_add_days(Days::new(1)).unwrap());
            ExpirableNonce::new(future_deadline, rng.random()).into()
        } else {
            rng.random()
        }
    }

    #[fixture]
    pub(crate) fn random_nonces(
        mut rng: impl Rng,
        #[default(10..1000)] size: impl RangeBounds<usize>,
    ) -> Vec<U256> {
        (0..range_to_random_size(&mut rng, size))
            .map(|_| generate_nonce(rng.random(), &mut rng))
            .collect()
    }

    fn get_legacy_map(nonces: &[U256], prefix: Vec<u8>) -> Nonces<LookupMap<U248, U256>> {
        let mut legacy_nonces = Nonces::new(LookupMap::new(prefix));
        for nonce in nonces {
            legacy_nonces
                .commit(*nonce)
                .expect("unable to commit nonce");
        }

        legacy_nonces
    }

    #[rstest]
    fn optimized_from_legacy(random_nonces: Vec<U256>, random_bytes: Vec<u8>) {
        let legacy_nonces = get_legacy_map(&random_nonces, random_bytes.clone());
        let optimized = MaybeOptimizedNonces::new_with_legacy(random_bytes, legacy_nonces);

        let legacy_map = optimized.legacy.as_ref().expect("No legacy nonces present");
        for nonce in &random_nonces {
            assert!(legacy_map.is_used(*nonce));
            assert!(!optimized.nonces.is_used(*nonce));
        }
    }

    #[rstest]
    #[allow(clippy::used_underscore_binding)]
    fn commit_new_nonce(random_bytes: Vec<u8>, mut rng: impl Rng) {
        let expirable_nonce = generate_nonce(true, &mut rng);
        let legacy_nonce = generate_nonce(false, &mut rng);
        let mut optimized = MaybeOptimizedNonces::new(random_bytes);

        optimized
            .commit_nonce(expirable_nonce)
            .expect("should be able to commit new expirable nonce");
        optimized
            .commit_nonce(legacy_nonce)
            .expect("should be able to commit new legacy nonce");

        assert!(optimized.nonces.is_used(expirable_nonce));
        assert!(optimized.nonces.is_used(legacy_nonce));
        assert!(optimized.legacy.is_none());
    }

    #[rstest]
    #[allow(clippy::used_underscore_binding)]
    fn commit_existg_legacy_nonce(random_nonces: Vec<U256>, random_bytes: Vec<u8>) {
        let legacy_nonces = get_legacy_map(&random_nonces, random_bytes.clone());
        let mut optimized = MaybeOptimizedNonces::new_with_legacy(random_bytes, legacy_nonces);

        assert!(matches!(
            optimized.commit_nonce(random_nonces[0]).unwrap_err(),
            DefuseError::NonceUsed
        ));
    }

    #[rstest]
    fn commit_duplicate_nonce(random_bytes: Vec<u8>, mut rng: impl Rng) {
        let mut optimized = MaybeOptimizedNonces::new(random_bytes);
        let nonce = generate_nonce(false, &mut rng);

        optimized
            .commit_nonce(nonce)
            .expect("First commit should succeed");

        assert!(matches!(
            optimized.commit_nonce(nonce).unwrap_err(),
            DefuseError::NonceUsed
        ));
    }

    #[rstest]
    fn commit_expired_nonce(random_bytes: Vec<u8>, mut rng: impl Rng) {
        let expired_deadline = Deadline::new(Utc::now().checked_sub_days(Days::new(1)).unwrap());
        let expired_nonce = ExpirableNonce::new(expired_deadline, rng.random()).into();

        let mut optimized = MaybeOptimizedNonces::new(random_bytes);

        assert!(matches!(
            optimized.commit_nonce(expired_nonce).unwrap_err(),
            DefuseError::NonceExpired
        ));
    }

    #[rstest]
    #[allow(clippy::used_underscore_binding)]
    fn expirable_nonces_searched_only_in_optimized_map(random_bytes: Vec<u8>, mut rng: impl Rng) {
        let expirable_nonce = generate_nonce(true, &mut rng);
        let nonces = get_legacy_map(&[expirable_nonce], random_bytes.clone());
        let optimized = MaybeOptimizedNonces::new_with_legacy(random_bytes, nonces);

        assert!(!optimized.is_nonce_used(expirable_nonce));
    }

    #[rstest]
    #[allow(clippy::used_underscore_binding)]
    fn check_used_nonces(
        #[from(make_arbitrary)] mut legacy_nonces: Vec<U256>,
        mut random_nonces: Vec<U256>,
        random_bytes: Vec<u8>,
    ) {
        let legacy_map = get_legacy_map(&legacy_nonces, random_bytes.clone());
        let mut optimized = MaybeOptimizedNonces::new_with_legacy(random_bytes, legacy_map);

        for nonce in &random_nonces {
            optimized
                .commit_nonce(*nonce)
                .expect("unable to commit nonce");
        }

        random_nonces.append(&mut legacy_nonces);

        for nonce in random_nonces {
            assert!(optimized.is_nonce_used(nonce));
        }
    }

    #[rstest]
    #[allow(clippy::used_underscore_binding)]
    fn legacy_nonces_cant_be_cleared(random_bytes: Vec<u8>, mut rng: impl Rng) {
        let random_nonce = generate_nonce(false, &mut rng);
        let legacy_nonces = get_legacy_map(&[random_nonce], random_bytes.clone());
        let mut optimized = MaybeOptimizedNonces::new_with_legacy(random_bytes, legacy_nonces);

        assert!(!optimized.clear_expired_nonce(random_nonce));
        assert!(optimized.is_nonce_used(random_nonce));
    }

    #[rstest]
    fn clear_active_nonce_fails(random_bytes: Vec<u8>, mut rng: impl Rng) {
        let future_deadline = Deadline::new(Utc::now().checked_add_days(Days::new(1)).unwrap());
        let valid_nonce = ExpirableNonce::new(future_deadline, rng.random()).into();

        let mut optimized = MaybeOptimizedNonces::new(random_bytes);

        assert!(!optimized.clear_expired_nonce(valid_nonce));
    }
}
