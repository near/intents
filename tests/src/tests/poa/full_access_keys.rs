use defuse_sandbox::{
    account::Account,
    extensions::{
        FnCallTransaction,
        acl::AccessControllableExt,
        poa::{
            PoAFactoryExt, PoaAddFullAccessKeyToTokensArgs, PoaFactory, PoaFactoryClient,
            PoaFactoryDeployerExt, contract::Role,
        },
    },
    kit::{AccessKeyPermissionView, Gas, KeyPair, Near, NearToken},
    root,
};
use defuse_test_utils::{asserts::ResultAssertsExt, wasms::POA_FACTORY_WASM};
use rstest::rstest;

async fn deploy_factory(root: &Near) -> PoaFactoryClient {
    root.deploy_poa_factory(
        "poa-factory",
        [root.account_id().clone()],
        [(Role::DAO, [root.account_id().clone()])],
        [
            (Role::DAO, [root.account_id().clone()]),
            (Role::TokenDeployer, [root.account_id().clone()]),
        ],
        POA_FACTORY_WASM.clone(),
    )
    .await
}

#[rstest]
#[tokio::test]
async fn add_full_access_key_to_tokens(#[future(awt)] root: Near) {
    let factory = deploy_factory(&root).await;
    let tokens = ["ft1", "ft2", "ft3"];
    let mut token_ids = Vec::new();
    for name in tokens {
        let token = root
            .poa_factory_deploy_token(factory.contract_id(), name, None)
            .await
            .unwrap();
        assert!(
            root.access_keys(token.contract_id())
                .await
                .unwrap()
                .keys
                .is_empty()
        );
        token_ids.push(token.contract_id().clone());
    }

    let public_key = KeyPair::random().public_key;
    let dao = root.create_subaccount("dao", NearToken::from_near(1)).await;
    root.acl_grant_role(factory.contract_id(), Role::DAO, dao.account_id())
        .await
        .unwrap();

    dao.fn_call(
        factory.contract_id(),
        PoaFactory::add_full_access_key_to_tokens(PoaAddFullAccessKeyToTokensArgs {
            public_key: public_key.clone(),
            tokens: tokens.into_iter().map(str::to_string).collect(),
        })
        .deposit(NearToken::from_yoctonear(1))
        .gas(Gas::from_tgas(100)),
    )
    .await
    .unwrap();

    // The factory detaches its promises, so verify the resulting keys on every token.
    for token_id in token_ids {
        let keys = root.access_keys(&token_id).await.unwrap().keys;
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].public_key, public_key);
        assert!(matches!(
            keys[0].access_key.permission,
            AccessKeyPermissionView::FullAccess
        ));
    }
    assert!(
        root.access_keys(factory.contract_id())
            .await
            .unwrap()
            .keys
            .iter()
            .all(|key| key.public_key != public_key)
    );
}

#[rstest]
#[tokio::test]
async fn add_full_access_key_to_tokens_requires_deployed_tokens(#[future(awt)] root: Near) {
    let factory = deploy_factory(&root).await;
    let token = root
        .poa_factory_deploy_token(factory.contract_id(), "ft1", None)
        .await
        .unwrap();

    let undeployed_token = "missing";
    root.fn_call(
        factory.contract_id(),
        PoaFactory::add_full_access_key_to_tokens(PoaAddFullAccessKeyToTokensArgs {
            public_key: KeyPair::random().public_key,
            tokens: vec!["ft1".to_string(), undeployed_token.to_string()],
        })
        .deposit(NearToken::from_yoctonear(1))
        .gas(Gas::from_tgas(100)),
    )
    .await
    .assert_err_contains(format!("token '{undeployed_token}' is not deployed"));

    assert!(
        root.access_keys(token.contract_id())
            .await
            .unwrap()
            .keys
            .is_empty()
    );
}
