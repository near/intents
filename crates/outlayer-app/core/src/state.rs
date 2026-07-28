use std::{borrow::Cow, collections::BTreeMap};

use near_account_id::AccountIdRef;

use crate::{AdminPublicKey, State};

/// An unreachable implicit account used as the admin for immutable instances.
pub const IMMUTABLE_ADMIN_ID: &AccountIdRef =
    AccountIdRef::new_or_panic("0000000000000000000000000000000000000000000000000000000000000000");

impl<'a> State<'a> {
    /// Create new state with given `admin_id` and `admin_public_key`.
    /// `code_hash` defaults to all-zero bytes and `code_url` to an empty string,
    /// until set via [`Self::with_code_hash`]/[`Self::with_code_url`].
    #[inline]
    pub fn new(
        admin_id: impl Into<Cow<'a, AccountIdRef>>,
        admin_public_key: AdminPublicKey,
    ) -> Self {
        Self {
            admin_id: admin_id.into(),
            code_hash: [0u8; 32],
            code_url: Cow::Borrowed(""),
            admin_public_key,
            state: BTreeMap::new(),
            config: BTreeMap::new(),
        }
    }

    /// Create an immutable state controlled by no reachable account.
    ///
    /// The admin ID is set to the all-zero implicit account ID.
    #[inline]
    pub fn new_immutable(admin_public_key: AdminPublicKey) -> Self {
        Self::new(IMMUTABLE_ADMIN_ID, admin_public_key)
    }

    /// Overwrite the approved code hash.
    #[must_use]
    #[inline]
    pub fn with_code_hash(mut self, code_hash: impl Into<[u8; 32]>) -> Self {
        self.code_hash = code_hash.into();
        self
    }

    /// Overwrite the code URL.
    #[must_use]
    #[inline]
    pub fn with_code_url(mut self, code_url: impl Into<Cow<'a, str>>) -> Self {
        self.code_url = code_url.into();
        self
    }

    /// Overwrite the initial state key-value pairs.
    #[must_use]
    #[inline]
    pub fn with_state<K, V>(mut self, state: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<Vec<u8>>,
        V: Into<Vec<u8>>,
    {
        self.state = state
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect();
        self
    }

    /// Overwrite the initial config key-value pairs.
    #[must_use]
    #[inline]
    pub fn with_config<K, V>(mut self, config: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<Vec<u8>>,
        V: Into<Vec<u8>>,
    {
        self.config = config
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect();
        self
    }

    #[cfg(feature = "digest")]
    /// Set [`Self::code_hash`] to the SHA-256 digest of the given code.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use defuse_crypto::ed25519::Ed25519PublicKey;
    /// # use defuse_outlayer_app_core::{AdminPublicKey, State};
    /// # use hex_literal::hex;
    /// # use near_account_id::AccountIdRef;
    /// # const ADMIN_ID: &AccountIdRef = AccountIdRef::new_or_panic("admin.near");
    /// # let admin_public_key = AdminPublicKey::Ed25519(Ed25519PublicKey([0u8; 32]));
    /// let wasm = b"TODO"; // read from file
    /// let state = State::new(ADMIN_ID, admin_public_key).with_code(wasm);
    ///
    /// assert_eq!(
    ///     state.code_hash,
    ///     hex!("337e547a950fc8a98592f10d964c1e79a304961790a8da0ce449a1f000cefabb"),
    /// )
    /// ```
    #[must_use]
    #[inline]
    pub fn with_code(self, code: impl AsRef<[u8]>) -> Self {
        use defuse_digest::{Digest, sha2::Sha256};

        self.with_code_hash(Sha256::digest(code))
    }

    /// Construct storage key-value pairs for `StateInit`
    /// of an Outlayer App contract.
    #[cfg(feature = "borsh")]
    #[inline]
    pub fn as_storage(&self) -> BTreeMap<Vec<u8>, Vec<u8>> {
        self.state_init()
    }
}

#[cfg(test)]
mod tests {
    use defuse_crypto::ed25519::Ed25519PublicKey;

    use super::*;

    #[test]
    fn immutable_state_uses_zero_implicit_admin() {
        let admin_public_key = AdminPublicKey::Ed25519(Ed25519PublicKey([0; 32]));
        let state = State::new_immutable(admin_public_key);

        assert_eq!(state.admin_id.as_ref(), IMMUTABLE_ADMIN_ID);
        assert!(state.admin_public_key.eq(&admin_public_key));
        assert_eq!(state.code_hash, [0; 32]);
        assert!(state.code_url.is_empty());
        assert!(state.state.is_empty());
        assert!(state.config.is_empty());
    }

    #[test]
    fn state_and_config_can_be_set_from_iterables() {
        let admin_public_key = AdminPublicKey::Ed25519(Ed25519PublicKey([0; 32]));
        let config = BTreeMap::from([(b"config-key".to_vec(), b"config-value".to_vec())]);

        let state = State::new_immutable(admin_public_key)
            .with_state([
                (b"state-key".as_slice(), b"first-value".as_slice()),
                (b"state-key".as_slice(), b"state-value".as_slice()),
            ])
            .with_config(config.clone());

        assert_eq!(
            state.state,
            BTreeMap::from([(b"state-key".to_vec(), b"state-value".to_vec())])
        );
        assert_eq!(state.config, config);
    }
}
