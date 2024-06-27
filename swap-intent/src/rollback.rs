use defuse_contracts::intents::swap::{IntentId, LostAsset, Rollback, SwapError, SwapIntentStatus};
use near_sdk::{env, near, NearToken, PromiseError, PromiseOrValue};

use crate::{SwapIntentContractImpl, SwapIntentContractImplExt};

#[near]
impl Rollback for SwapIntentContractImpl {
    #[payable]
    fn rollback_intent(&mut self, id: &IntentId) -> PromiseOrValue<bool> {
        assert_eq!(env::attached_deposit(), NearToken::from_yoctonear(1));
        self.internal_rollback_intent(id).unwrap()
    }
}

impl SwapIntentContractImpl {
    fn internal_rollback_intent(
        &mut self,
        id: &IntentId,
    ) -> Result<PromiseOrValue<bool>, SwapError> {
        let intent = self
            .intents
            .get_mut(id)
            .ok_or_else(|| SwapError::NotFound(id.clone()))?
            .lock()
            .ok_or(SwapError::Locked)?
            .as_available()
            .ok_or(SwapError::WrongStatus)?;

        // TODO: only initiator

        assert!(
            env::prepaid_gas().saturating_sub(env::used_gas())
                >= intent.asset_in.gas_for_transfer()
        );

        // TODO: emit log

        Ok(
            Self::transfer(id, intent.asset_in.clone(), intent.initiator.clone())
                .then(Self::ext(env::current_account_id()).resolve_rollback_intent(id))
                .into(),
        )
    }
}

#[near]
impl SwapIntentContractImpl {
    #[private]
    pub fn resolve_rollback_intent(
        &mut self,
        id: &IntentId,
        #[callback_result] transfer_asset_in: Result<(), PromiseError>,
    ) -> bool {
        self.internal_resolve_rollback_intent(id, transfer_asset_in)
            .unwrap()
    }
}

impl SwapIntentContractImpl {
    fn internal_resolve_rollback_intent(
        &mut self,
        id: &IntentId,
        transfer_asset_in: Result<(), PromiseError>,
    ) -> Result<bool, SwapError> {
        let intent = self
            .intents
            .get_mut(id)
            .ok_or_else(|| SwapError::NotFound(id.clone()))?
            .unlock()
            .ok_or(SwapError::Unlocked)?;

        let swap = intent.as_available().ok_or(SwapError::WrongStatus)?.clone();

        if transfer_asset_in.is_ok() {
            self.intents.remove(id);
        } else {
            // TODO: log
            *intent = SwapIntentStatus::Lost(LostAsset {
                asset: swap.asset_in,
                recipient: swap.initiator,
            });
        }
        Ok(transfer_asset_in.is_ok())
    }
}
