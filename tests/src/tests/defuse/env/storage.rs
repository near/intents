use crate::{
    tests::defuse::{
        accounts::AccountManagerExt,
        env::{Env, arbitrary_state::ArbitraryState},
        state::{FeesManagerExt, SaltManagerExt},
    },
    utils::mt::MtExt,
};

pub trait StorageMigration {
    async fn apply_storage_data(&self, state: &ArbitraryState);
    async fn verify_storage_consistency(&self, state: &ArbitraryState);
}

impl StorageMigration for Env {
    async fn apply_storage_data(&self, arbitrary_state: &ArbitraryState) {
        // self.defuse
        //     .execute_intents(&arbitrary_state.intents)
        //     .await
        //     .expect(|e| anyhow!("failed to execute intent: {:?}", e));
    }

    async fn verify_storage_consistency(&self, state: &ArbitraryState) {
        let fee = self.defuse.fee(&self.defuse.id()).await.unwrap();
        assert_eq!(fee, state.fees.fee);

        let fee_collector = self.defuse.fee_collector(&self.defuse.id()).await.unwrap();
        assert_eq!(fee_collector, state.fees.fee_collector);

        for (salt, is_valid) in &state.salts {
            let valid = self
                .defuse
                .is_valid_salt(&self.defuse.id(), salt)
                .await
                .unwrap();

            assert_eq!(valid, *is_valid);
        }

        for data in &state.accounts {
            let enabled = self
                .defuse
                .is_auth_by_predecessor_id_enabled(&data.account_id)
                .await
                .unwrap();

            assert_eq!(data.disable_auth_by_predecessor, !enabled);

            for pubkey in &data.public_keys {
                assert!(
                    self.defuse
                        .has_public_key(&data.account_id, pubkey)
                        .await
                        .unwrap()
                );
            }

            for nonce in &data.nonces {
                assert!(
                    self.defuse
                        .is_nonce_used(&data.account_id, nonce)
                        .await
                        .unwrap()
                );
            }

            let tokens = &data
                .token_balances
                .keys()
                .map(|t| t.to_string())
                .collect::<Vec<String>>();

            let balances = self
                .defuse
                .mt_batch_balance_of(&data.account_id, tokens)
                .await
                .unwrap();

            for (pos, (_, amount)) in data.token_balances.iter().enumerate() {
                let balance = balances.get(pos).unwrap();
                assert_eq!(*balance, *amount);
            }
        }
    }
}
