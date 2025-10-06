use defuse_bitmap::{U248, U256};

use defuse_map_utils::Map;
use near_sdk::{
    near,
    store::{LookupMap, key::Sha256},
};

use defuse_core::{DefuseError, Nonce, Nonces, Result};

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
        // Check legacy map only if the nonce is not expirable
        // otherwise check both maps

        // TODO: legacy nonces which have expirable prefix can be committed twice, check probability!
        self.nonces.is_used(nonce)
            || self
                .legacy
                .as_ref()
                .is_some_and(|legacy| legacy.is_used(nonce))
    }

    #[inline]
    pub fn clear_expired(&mut self, nonce: Nonce) -> bool {
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
        #[default(10..100)] size: impl RangeBounds<usize>,
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
    fn new_from_legacy(random_nonces: Vec<U256>, random_bytes: Vec<u8>) {
        let legacy_nonces = get_legacy_map(&random_nonces, random_bytes.clone());
        let new = MaybeLegacyAccountNonces::with_legacy(
            legacy_nonces,
            LookupMap::with_hasher(random_bytes),
        );

        let legacy_map = new.legacy.as_ref().expect("No legacy nonces present");

        for nonce in &random_nonces {
            assert!(legacy_map.is_used(*nonce));
            assert!(!new.nonces.is_used(*nonce));
            assert!(new.is_used(*nonce));
        }
    }

    #[rstest]
    #[allow(clippy::used_underscore_binding)]
    fn commit_new_nonce(random_bytes: Vec<u8>, mut rng: impl Rng) {
        let expirable_nonce = generate_nonce(true, &mut rng);
        let legacy_nonce = generate_nonce(false, &mut rng);
        let mut new = MaybeLegacyAccountNonces::new(LookupMap::with_hasher(random_bytes));

        new.commit(expirable_nonce)
            .expect("should be able to commit new expirable nonce");
        new.commit(legacy_nonce)
            .expect("should be able to commit new legacy nonce");

        assert!(new.legacy.is_none());

        for n in [expirable_nonce, legacy_nonce] {
            assert!(new.nonces.is_used(n));
            assert!(new.is_used(n));
        }
    }

    #[rstest]
    #[allow(clippy::used_underscore_binding)]
    fn commit_existing_legacy_nonce(random_nonces: Vec<U256>, random_bytes: Vec<u8>) {
        let legacy_nonces = get_legacy_map(&random_nonces, random_bytes.clone());
        let mut new = MaybeLegacyAccountNonces::with_legacy(
            legacy_nonces,
            LookupMap::with_hasher(random_bytes),
        );

        assert!(matches!(
            new.commit(random_nonces[0]).unwrap_err(),
            DefuseError::NonceUsed
        ));
    }

    #[rstest]
    fn commit_duplicate_nonce(random_bytes: Vec<u8>, mut rng: impl Rng) {
        let mut new = MaybeLegacyAccountNonces::new(LookupMap::with_hasher(random_bytes));
        let nonce = generate_nonce(false, &mut rng);

        new.commit(nonce).expect("First commit should succeed");

        assert!(matches!(
            new.commit(nonce).unwrap_err(),
            DefuseError::NonceUsed
        ));
    }

    #[rstest]
    fn commit_expired_nonce(random_bytes: Vec<u8>, mut rng: impl Rng) {
        let expired_deadline = Deadline::new(Utc::now().checked_sub_days(Days::new(1)).unwrap());
        let expired_nonce = ExpirableNonce::new(expired_deadline, rng.random()).into();

        let mut new = MaybeLegacyAccountNonces::new(LookupMap::with_hasher(random_bytes));

        assert!(matches!(
            new.commit(expired_nonce).unwrap_err(),
            DefuseError::NonceExpired
        ));
    }

    #[rstest]
    #[allow(clippy::used_underscore_binding)]
    fn check_used_nonces(
        #[from(make_arbitrary)] legacy_nonces: Vec<U256>,
        random_nonces: Vec<U256>,
        random_bytes: Vec<u8>,
    ) {
        let legacy_map = get_legacy_map(&legacy_nonces, random_bytes.clone());
        let mut new =
            MaybeLegacyAccountNonces::with_legacy(legacy_map, LookupMap::with_hasher(random_bytes));

        for nonce in &random_nonces {
            new.commit(*nonce).expect("unable to commit nonce");
        }

        for nonce in random_nonces.iter().chain(&legacy_nonces) {
            assert!(new.is_used(*nonce));
        }
    }

    #[rstest]
    #[allow(clippy::used_underscore_binding)]
    fn legacy_nonces_cant_be_cleared(
        #[values(true, false)] expirable: bool,
        random_bytes: Vec<u8>,
        mut rng: impl Rng,
    ) {
        let random_nonce = generate_nonce(expirable, &mut rng);
        let legacy_nonces = get_legacy_map(&[random_nonce], random_bytes.clone());
        let mut new = MaybeLegacyAccountNonces::with_legacy(
            legacy_nonces,
            LookupMap::with_hasher(random_bytes),
        );

        assert!(!new.clear_expired(random_nonce));
        assert!(new.is_used(random_nonce));
    }

    #[rstest]
    fn clear_active_nonce_fails(random_bytes: Vec<u8>, mut rng: impl Rng) {
        let future_deadline = Deadline::new(Utc::now().checked_add_days(Days::new(1)).unwrap());
        let valid_nonce = ExpirableNonce::new(future_deadline, rng.random()).into();

        let mut new = MaybeLegacyAccountNonces::new(LookupMap::with_hasher(random_bytes));

        new.commit(valid_nonce)
            .expect("should be able to commit new expirable nonce");

        assert!(!new.clear_expired(valid_nonce));
        assert!(new.is_used(valid_nonce));
    }
}
