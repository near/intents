mod accounts;
mod env;
mod intents;
mod state;
mod storage;
mod tokens;
use defuse::core::ExpirableNonce;
use defuse::core::SaltedNonce;
use defuse::core::VersionedNonce;
use defuse::core::intents::DefuseIntents;
use defuse::sandbox_ext::signer::DefuseSigner;
use defuse::sandbox_ext::state::SaltViewExt;
use defuse_randomness::RngCore;

use defuse::core::intents::Intent;
use defuse::core::{Deadline, Nonce, payload::multi::MultiPayload};
use defuse_sandbox::Account;
use defuse_test_utils::random::TestRng;

#[allow(async_fn_in_trait)]
pub trait DefuseSignerExt: DefuseSigner {
    async fn unique_nonce(
        &self,
        defuse_contract: &Account,
        deadline: Option<Deadline>,
    ) -> anyhow::Result<Nonce> {
        let deadline =
            deadline.unwrap_or_else(|| Deadline::timeout(std::time::Duration::from_secs(120)));

        let salt = defuse_contract
            .current_salt()
            .await
            .expect("should be able to fetch salt");

        let mut nonce_bytes = [0u8; 15];
        TestRng::from_entropy().fill_bytes(&mut nonce_bytes);

        let salted = SaltedNonce::new(salt, ExpirableNonce::new(deadline, nonce_bytes));
        Ok(VersionedNonce::V1(salted).into())
    }

    async fn sign_defuse_payload_default<T>(
        &self,
        defuse_contract: &Account,
        intents: impl IntoIterator<Item = T>,
    ) -> anyhow::Result<MultiPayload>
    where
        T: Into<Intent>,
    {
        let deadline = Deadline::timeout(std::time::Duration::from_secs(120));
        let nonce = self.unique_nonce(defuse_contract, Some(deadline)).await?;

        let defuse_intents = DefuseIntents {
            intents: intents.into_iter().map(Into::into).collect(),
        };
        Ok(self
            .sign_defuse_message(defuse_contract.id(), nonce, deadline, defuse_intents)
            .await)
    }
}
impl<T> DefuseSignerExt for T where T: DefuseSigner {}
