use crate::tests::defuse::DefuseSignerExt;
use crate::tests::defuse::{env::Env, intents::ExecuteIntentsExt};
use crate::utils::fixtures::{ed25519_pk, secp256k1_pk};
use defuse::sandbox_ext::tokens::nep141::DefuseFtDepositor;
use defuse::{
    core::{
        crypto::PublicKey,
        intents::tokens::NativeWithdraw,
        token_id::{TokenId, nep141::Nep141TokenId},
    },
    tokens::DepositMessage,
};
use defuse_sandbox::extensions::mt::MtViewExt;
use defuse_sandbox::extensions::wnear::WNearExt;
use near_sdk::NearToken;
use rstest::rstest;

#[tokio::test]
#[rstest]
async fn native_withdraw_intent(ed25519_pk: PublicKey, secp256k1_pk: PublicKey) {
    let env = Env::new().await;

    let (user, other_user) = futures::join!(env.create_user(), env.create_user());

    env.initial_ft_storage_deposit(vec![user.id(), other_user.id()], &[])
        .await;

    let amounts_to_withdraw = [
        // Check for different account_id types
        // See https://github.com/near/nearcore/blob/dcfb6b9fb9f896b839b8728b8033baab963de344/core/parameters/src/cost.rs#L691-L709
        (
            ed25519_pk.to_implicit_account_id(),
            NearToken::from_near(100),
        ),
        (
            secp256k1_pk.to_implicit_account_id(),
            NearToken::from_near(200),
        ),
        (user.id().to_owned(), NearToken::from_near(300)),
    ];

    let initial_balances = {
        let mut result = vec![];
        for (account, _) in &amounts_to_withdraw {
            let balance = env
                .account(account)
                .view()
                .await
                .map(|a| a.amount)
                .unwrap_or(NearToken::from_near(0));

            result.push(balance);
        }
        result
    };

    let total_amount_yocto = amounts_to_withdraw
        .iter()
        .map(|(_, amount)| amount.as_yoctonear())
        .sum();

    env.near_deposit(
        env.wnear.id(),
        NearToken::from_yoctonear(total_amount_yocto),
    )
    .await
    .expect("failed to wrap NEAR");

    env.defuse_ft_deposit(
        env.defuse.id(),
        env.wnear.id(),
        total_amount_yocto,
        DepositMessage::new(other_user.id().clone()),
    )
    .await
    .expect("failed to deposit wNEAR to user2");

    // withdraw native NEAR to corresponding receivers
    let withdraw_payload = other_user
        .sign_defuse_payload_default(
            &env.defuse,
            amounts_to_withdraw
                .iter()
                .cloned()
                .map(|(receiver_id, amount)| NativeWithdraw {
                    receiver_id,
                    amount,
                }),
        )
        .await
        .unwrap();

    env.simulate_and_execute_intents(env.defuse.id(), [withdraw_payload])
        .await
        .expect("execute_intents: failed to withdraw native NEAR to receivers");

    assert_eq!(
        env.defuse
            .mt_balance_of(
                user.id(),
                &TokenId::Nep141(Nep141TokenId::new(env.wnear.id().clone())).to_string()
            )
            .await
            .unwrap(),
        0,
        "there should be nothing left deposited for user1"
    );

    // Check balances of NEAR on the blockchain
    for ((receiver_id, amount), initial_balance) in amounts_to_withdraw.iter().zip(initial_balances)
    {
        let balance = env.account(receiver_id).view().await.unwrap().amount;

        assert!(
            balance == initial_balance.checked_add(*amount).unwrap(),
            "wrong NEAR balance for {receiver_id}: expected minimum {amount}, got {balance}"
        );
    }
}
