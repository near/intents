use defuse_sandbox::{
    account::Account,
    extensions::poa::{
        IdDigest, PayloadHash, PoAFactoryExt, PoaFactoryClient, PoaFactoryDeployerExt,
        PoaGetWithdrawalArgs, Withdrawal, contract::Role,
    },
    kit::{Near, NearToken},
    root,
};
use defuse_test_utils::wasms::POA_FACTORY_WASM;
use futures::try_join;
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn deploy_mint(#[future(awt)] root: Near) {
    let user = root
        .create_subaccount("user1", NearToken::from_near(10))
        .await;

    let poa_factory = root
        .deploy_poa_factory(
            "poa-factory",
            [root.account_id().clone()],
            [
                (Role::TokenDeployer, [root.account_id().clone()]),
                (Role::TokenDepositer, [root.account_id().clone()]),
            ],
            [
                (Role::TokenDeployer, [root.account_id().clone()]),
                (Role::TokenDepositer, [root.account_id().clone()]),
            ],
            POA_FACTORY_WASM.clone(),
        )
        .await;

    user.poa_factory_deploy_token(poa_factory.contract_id(), "ft1", None)
        .await
        .unwrap_err();

    root.poa_factory_deploy_token(poa_factory.contract_id(), "ft1.abc", None)
        .await
        .unwrap_err();

    let ft1 = root
        .poa_factory_deploy_token(poa_factory.contract_id(), "ft1", None)
        .await
        .unwrap();

    root.poa_factory_deploy_token(poa_factory.contract_id(), "ft1", None)
        .await
        .unwrap_err();

    assert!(ft1.balance_of(user.account_id()).await.unwrap().is_zero());

    try_join!(
        ft1.storage_deposit(root.account_id(), NearToken::from_near(1))
            .into_future(),
        ft1.storage_deposit(user.account_id(), NearToken::from_near(1))
            .into_future()
    )
    .unwrap();

    user.poa_factory_ft_deposit(
        poa_factory.contract_id(),
        "ft1",
        user.account_id(),
        1000,
        None,
        None,
    )
    .await
    .unwrap_err();

    root.poa_factory_ft_deposit(
        poa_factory.contract_id(),
        "ft1",
        user.account_id(),
        1000,
        None,
        None,
    )
    .await
    .unwrap();

    let balance: u128 = ft1.balance_of(user.account_id()).await.unwrap().into();

    assert_eq!(balance, 1000);
}

async fn deploy_factory_with_all_roles(root: &Near) -> PoaFactoryClient {
    root.deploy_poa_factory(
        "poa-factory",
        [root.account_id().clone()],
        [
            (Role::DAO, [root.account_id().clone()]),
            (Role::TokenDeployer, [root.account_id().clone()]),
            (Role::TokenDepositer, [root.account_id().clone()]),
            (Role::OmniProver, [root.account_id().clone()]),
        ],
        [
            (Role::DAO, [root.account_id().clone()]),
            (Role::TokenDeployer, [root.account_id().clone()]),
            (Role::TokenDepositer, [root.account_id().clone()]),
            (Role::OmniProver, [root.account_id().clone()]),
        ],
        POA_FACTORY_WASM.clone(),
    )
    .await
}

fn sample_withdrawal(payload_hash: [u8; 32], metadata: &str) -> Withdrawal {
    Withdrawal {
        payload_hash: payload_hash.into(),
        metadata: metadata.to_string(),
    }
}

/// Callers digest their own ids, so tests just pick distinct digests.
fn id(byte: u8) -> IdDigest {
    IdDigest([byte; IdDigest::LEN])
}

#[rstest]
#[tokio::test]
async fn record_withdraw_and_reject_duplicate(#[future(awt)] root: Near) {
    let unauthorized = root
        .create_subaccount("unauth", NearToken::from_near(10))
        .await;
    let poa_factory = deploy_factory_with_all_roles(&root).await;

    let withdrawal = sample_withdrawal([1u8; 32], "meta-1");
    let w1 = id(1);

    unauthorized
        .poa_factory_record_withdraw(poa_factory.contract_id(), w1, withdrawal.clone())
        .await
        .unwrap_err();

    root.poa_factory_record_withdraw(poa_factory.contract_id(), w1, withdrawal.clone())
        .await
        .unwrap();

    let stored = poa_factory
        .get_withdrawal(PoaGetWithdrawalArgs { withdrawal_id: w1 })
        .await
        .unwrap()
        .expect("withdrawal must be stored");
    assert_eq!(stored.payload_hash, withdrawal.payload_hash);
    assert_eq!(stored.metadata, withdrawal.metadata);

    let err = root
        .poa_factory_record_withdraw(poa_factory.contract_id(), w1, withdrawal.clone())
        .await
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("withdrawal already exists"),
        "unexpected error: {err:?}"
    );

    root.poa_factory_remove_withdrawals(poa_factory.contract_id(), vec![w1])
        .await
        .unwrap();
    assert!(
        poa_factory
            .get_withdrawal(PoaGetWithdrawalArgs { withdrawal_id: w1 })
            .await
            .unwrap()
            .is_none()
    );
}

#[rstest]
#[tokio::test]
async fn update_withdraw_record(#[future(awt)] root: Near) {
    let unauthorized = root
        .create_subaccount("unauth-upd", NearToken::from_near(10))
        .await;
    let poa_factory = deploy_factory_with_all_roles(&root).await;

    let w_upd = id(2);
    let original = sample_withdrawal([9u8; 32], "meta-orig");
    root.poa_factory_record_withdraw(poa_factory.contract_id(), w_upd, original.clone())
        .await
        .unwrap();

    let prev_hash = original.payload_hash;
    let updated_hash = PayloadHash([5u8; 32]);
    let updated_metadata = "meta-updated".to_string();

    unauthorized
        .poa_factory_update_withdraw_record(
            poa_factory.contract_id(),
            w_upd,
            prev_hash,
            updated_hash,
            updated_metadata.clone(),
        )
        .await
        .unwrap_err();

    let wrong_prev = PayloadHash([0u8; 32]);
    let err = root
        .poa_factory_update_withdraw_record(
            poa_factory.contract_id(),
            w_upd,
            wrong_prev,
            updated_hash,
            updated_metadata.clone(),
        )
        .await
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("payload hash mismatch"),
        "unexpected error: {err:?}"
    );

    let err = root
        .poa_factory_update_withdraw_record(
            poa_factory.contract_id(),
            id(0xEE),
            prev_hash,
            updated_hash,
            updated_metadata.clone(),
        )
        .await
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("withdrawal not found"),
        "unexpected error: {err:?}"
    );

    root.poa_factory_update_withdraw_record(
        poa_factory.contract_id(),
        w_upd,
        prev_hash,
        updated_hash,
        updated_metadata.clone(),
    )
    .await
    .unwrap();

    let stored = poa_factory
        .get_withdrawal(PoaGetWithdrawalArgs {
            withdrawal_id: w_upd,
        })
        .await
        .unwrap()
        .expect("withdrawal must still exist");
    assert_eq!(stored.payload_hash, updated_hash);
    assert_eq!(stored.metadata, updated_metadata);
}
