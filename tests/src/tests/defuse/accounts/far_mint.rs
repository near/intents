use std::{borrow::Cow, str::FromStr};

use defuse::{
    contract::Role,
    nep245::{MtEvent, MtMintEvent},
};
use defuse_escrow_swap::token_id::{TokenId, nep141::Nep141TokenId};
use defuse_sandbox::{
    assert_eq_event_logs,
    extensions::{acl::AclExt, mt::MtViewExt},
    tx::FnCallBuilder,
};
use defuse_test_utils::asserts::ResultAssertsExt;
use near_sdk::AsNep297Event;
use near_sdk::NearToken;
use near_sdk::json_types::U128;
use rstest::rstest;
use serde_json::json;

use crate::tests::defuse::env::Env;

#[rstest]
#[trace]
#[tokio::test]
async fn test_far_mint() {
    let env = Env::builder().deployer_as_super_admin().build().await;

    let (user1, user2) = futures::join!(env.create_user(), env.create_user());
    let ft_token = TokenId::Nep141(Nep141TokenId::from_str("sometoken.near").unwrap());
    let amount = U128::from(100);
    // only DAO or mint manager can mint tokens
    {
        user1
            .tx(env.defuse.id().clone())
            .function_call(
                FnCallBuilder::new("mint_tokens")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "receiver_id": user2.id(),
                        "token_id": ft_token,
                        "amount": amount
                    })),
            )
            .exec_transaction()
            .await
            .unwrap()
            .into_result()
            .assert_err_contains("Insufficient permissions for method");
    }

    // Mint ft tokens
    {
        env.acl_grant_role(env.defuse.id(), Role::FarMintManager, user1.id())
            .await
            .expect("failed to grant role");

        let result = user1
            .tx(env.defuse.id().clone())
            .function_call(
                FnCallBuilder::new("mint_tokens")
                    .with_deposit(NearToken::from_yoctonear(1))
                    .json_args(json!({
                        "receiver_id": user2.id(),
                        "token_id": ft_token,
                        "amount": amount
                    })),
            )
            .exec_transaction()
            .await
            .unwrap()
            .into_result()
            .unwrap();

        assert_eq_event_logs!(
            result.logs().clone(),
            [MtEvent::MtMint(Cow::Owned(vec![MtMintEvent {
                owner_id: user2.id().into(),
                token_ids: vec![ft_token.to_string()].into(),
                amounts: vec![amount].into(),
                memo: Some("mint".to_string().into()),
            }]))
            .to_nep297_event()
            .to_event_log(),]
        );

        assert_eq!(
            env.defuse
                .mt_balance_of(user2.id(), &ft_token.to_string())
                .await
                .unwrap(),
            amount.0
        );
    }
}
