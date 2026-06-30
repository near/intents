use defuse_sandbox::kit::{self, Action, Final, UseGlobalContractAction};
use defuse_test_utils::wasms::WALLET_NO_SIGN_WASM;
use rstest::rstest;

use super::*;

#[rstest]
#[awt]
#[tokio::test]
async fn test_w_init(
    #[future]
    #[with(WALLET_NO_SIGN_WASM.clone())]
    env: Env,
) {
    let user = env
        .create_subaccount("user", NearToken::from_near(10))
        .await;

    // initialize wallet contract on existing account
    user.transaction(user.account_id())
        .add_action(Action::UseGlobalContract(UseGlobalContractAction {
            contract_identifier: env.wallet_global_id.clone(),
        }))
        .add_action(
            kit::FunctionCall::new("w_init")
                .gas(Gas::from_tgas(5))
                .deposit(NearToken::from_yoctonear(1)),
        )
        .wait_until(Final)
        .await
        .unwrap()
        .result()
        .unwrap();

    user.w_execute_extension(
        user.account_id(),
        None,
        &Request::new().ops([WalletOp::RemoveExtension {
            account_id: user.account_id().into(),
        }]),
        NearToken::from_yoctonear(1),
    )
    .await
    .expect_err("cannot accidentally delete itself from extension");

    let user2 = env
        .create_subaccount("user2", NearToken::from_near(1))
        .await;

    // add another account as an extension
    user.w_execute_extension(
        user.account_id(),
        None,
        &Request::new().ops([WalletOp::AddExtension {
            account_id: user2.account_id().into(),
        }]),
        NearToken::from_yoctonear(1),
    )
    .await
    .unwrap();

    let receiver = env.create_implicit(None).await;

    // send extension request from user2
    user2
        .w_execute_extension(
            user.account_id(),
            None,
            &Request::new()
                .out([NearPromise::new(receiver.account_id()).transfer(NearToken::from_near(5))]),
            NearToken::from_yoctonear(1),
        )
        .await
        .unwrap();

    assert_eq!(
        env.balance(receiver.account_id())
            .await
            .expect("implicit account should be created by incoming transfer")
            .total,
        NearToken::from_near(5),
    );
}
