use defuse_near_utils::UnwrapOrPanicError;
use defuse_serde_utils::hex::AsHex;
use near_sdk::{
    AccountId, Gas, NearToken, PanicOnDefault, Promise, assert_one_yocto, env, near, require,
};

use crate::{
    ApprovedDeployments, Deployment, Event, GlobalDeployer, State,
    error::{
        ERR_INSUFFICIENT_DEPOSIT, ERR_SAME_CODE, ERR_SELF_TRANSFER, ERR_UNAUTHORIZED,
        ERR_WRONG_CODE_HASH,
    },
};

const GD_AT_DEPLOY_GAS: Gas = Gas::from_tgas(15);

#[near(
    contract_state(key = State::STATE_KEY),
    contract_metadata(
        standard(standard = "global-deployer", version = "1.0.0")
    )
)]
#[derive(PanicOnDefault)]
pub struct Contract(State);

#[near]
impl GlobalDeployer for Contract {
    fn gd_approve(&mut self, deployment: Deployment) {
        self.require_owner();

        ApprovedDeployments::approve(&deployment, &self.0.owner_id).unwrap_or_panic_display();

        Event::DeploymentApproved {
            deployment_hash: deployment.hash(),
            new_hash: deployment.new_hash,
        }
        .emit();
    }

    fn gd_revoke(&mut self, deployments: Vec<Deployment>) {
        let caller = env::predecessor_account_id();
        let hashes: Vec<[u8; 32]> = deployments.iter().map(|u| u.hash()).collect();

        ApprovedDeployments::revoke(&deployments, &caller, &self.0.owner_id)
            .unwrap_or_panic_display();

        Event::DeploymentsRevoked { hashes }.emit();
    }

    #[payable]
    fn gd_exec_approved_deployment(
        &mut self,
        #[serializer(borsh)] deployment: Deployment,
        #[serializer(borsh)] new_code: Vec<u8>,
    ) -> Promise {
        require!(!env::attached_deposit().is_zero(), ERR_INSUFFICIENT_DEPOSIT);

        let actual_code_hash = env::sha256_array(&new_code);

        let caller = env::predecessor_account_id();
        ApprovedDeployments::check(
            &deployment,
            &caller,
            &self.0.owner_id,
            &self.0.code_hash,
            &actual_code_hash,
        )
        .unwrap_or_panic_display();

        let deployment_hash = deployment.hash();
        let old_hash = self.0.code_hash;
        let initial_balance = env::account_balance().saturating_sub(env::attached_deposit());

        Self::ext_on(
            Promise::new(env::current_account_id())
                .refund_to(env::refund_to_account_id())
                .transfer(env::attached_deposit())
                .deploy_global_contract_by_account_id(new_code),
        )
        .with_static_gas(GD_AT_DEPLOY_GAS)
        .with_unused_gas_weight(1)
        .gd_post_deploy(
            old_hash.into(),
            deployment.new_hash.into(),
            initial_balance,
            Some(deployment_hash.into()),
        )
    }

    #[payable]
    fn gd_deploy(
        &mut self,
        #[serializer(borsh)] old_hash: [u8; 32],
        #[serializer(borsh)] new_code: Vec<u8>,
    ) -> Promise {
        require!(!env::attached_deposit().is_zero(), ERR_INSUFFICIENT_DEPOSIT);
        self.require_owner();

        require!(self.0.code_hash == old_hash, ERR_WRONG_CODE_HASH);
        let new_hash = env::sha256_array(&new_code);
        require!(new_hash != old_hash, ERR_SAME_CODE);

        let initial_balance = env::account_balance().saturating_sub(env::attached_deposit());

        Self::ext_on(
            Promise::new(env::current_account_id())
                .refund_to(env::refund_to_account_id())
                .transfer(env::attached_deposit())
                .deploy_global_contract_by_account_id(new_code),
        )
        .with_static_gas(GD_AT_DEPLOY_GAS)
        .with_unused_gas_weight(1)
        .gd_post_deploy(old_hash.into(), new_hash.into(), initial_balance, None)
    }

    #[payable]
    fn gd_transfer_ownership(&mut self, receiver_id: AccountId) {
        assert_one_yocto();
        self.require_owner();

        require!(self.0.owner_id != receiver_id, ERR_SELF_TRANSFER);
        Event::Transfer {
            old_owner_id: (&self.0.owner_id).into(),
            new_owner_id: (&receiver_id).into(),
        }
        .emit();
        self.0.owner_id = receiver_id;
    }

    fn gd_owner_id(&self) -> AccountId {
        self.0.owner_id.clone()
    }

    fn gd_index(&self) -> u32 {
        self.0.index
    }

    fn gd_code_hash(&self) -> AsHex<[u8; 32]> {
        self.0.code_hash.into()
    }

    fn gd_as_borsh(&self, deployment: Deployment) -> Vec<u8> {
        borsh::to_vec(&deployment).unwrap_or_else(|_| unreachable!())
    }
}

#[near]
impl Contract {
    #[private]
    pub fn gd_post_deploy(
        &mut self,
        old_hash: AsHex<[u8; 32]>,
        new_hash: AsHex<[u8; 32]>,
        initial_balance: NearToken,
        approval_hash: Option<AsHex<[u8; 32]>>,
    ) {
        let [old_hash, new_hash] = [old_hash, new_hash].map(AsHex::into_inner);

        require!(self.0.code_hash == old_hash, ERR_WRONG_CODE_HASH);
        self.0.code_hash = new_hash;
        Event::Deploy { old_hash, new_hash }.emit();

        if let Some(deployment_hash) = approval_hash {
            let deployment_hash = deployment_hash.into_inner();
            if ApprovedDeployments::take(&deployment_hash).is_ok() {
                Event::ApprovedDeploymentExecuted { deployment_hash }.emit();
            }
        }

        let refund = env::account_balance().saturating_sub(initial_balance);
        if !refund.is_zero() {
            Promise::new(env::refund_to_account_id())
                .transfer(refund)
                .detach();
        }
    }
}

impl Contract {
    fn require_owner(&self) {
        require!(
            env::predecessor_account_id() == self.0.owner_id,
            ERR_UNAUTHORIZED
        );
    }
}
