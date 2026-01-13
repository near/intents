use defuse::core::token_id::TokenId as DefuseTokenId;
use defuse::core::token_id::nep171::Nep171TokenId;
use defuse::tokens::{DepositAction, DepositMessage, ExecuteIntents};

use defuse_sandbox::api::types::nft::NFTContractMetadata;
use defuse_sandbox::extensions::defuse::signer::DefaultDefuseSignerExt;
use defuse_sandbox::extensions::mt::MtViewExt;
use defuse_sandbox::extensions::nft::{NftDeployerExt, NftExt, NftViewExt};
use multi_token_receiver_stub::MTReceiverMode as StubAction;
use near_contract_standards::non_fungible_token::metadata::NFT_METADATA_SPEC;
use near_contract_standards::non_fungible_token::{Token, metadata::TokenMetadata};
use near_sdk::NearToken;
use rstest::rstest;

use crate::MT_RECEIVER_STUB_WASM;
use defuse_tests::env::Env;

const DUMMY_REFERENCE_HASH: [u8; 32] = [33; 32];
const DUMMY_NFT1_ID: &str = "thisisdummynftid1";

#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
struct NftTransferCallExpectation {
    action: StubAction,
    intent_transfer: bool,
    refund_if_fails: bool,
    expected_sender_owns_nft: bool,
    expected_receiver_owns_nft: bool,
}

#[rstest]
#[case::nothing_to_refund(NftTransferCallExpectation {
    action: StubAction::ReturnValue(0.into()),
    intent_transfer: false,
    refund_if_fails: true,
    expected_sender_owns_nft: false,
    expected_receiver_owns_nft: true,
})]
#[case::request_refund(NftTransferCallExpectation {
    action: StubAction::ReturnValue(1.into()),
    intent_transfer: false,
    refund_if_fails: true,
    expected_sender_owns_nft: true,
    expected_receiver_owns_nft: false,
})]
#[case::receiver_panics(NftTransferCallExpectation {
    action: StubAction::Panic,
    intent_transfer: false,
    refund_if_fails: true,
    expected_sender_owns_nft: true,
    expected_receiver_owns_nft: false,
})]
#[case::malicious_receiver(NftTransferCallExpectation {
    action: StubAction::MaliciousReturn,
    intent_transfer: false,
    refund_if_fails: true,
    expected_sender_owns_nft: true,
    expected_receiver_owns_nft: false,
})]
#[tokio::test]
async fn nft_transfer_call_calls_mt_on_transfer_variants(
    #[case] expectation: NftTransferCallExpectation,
) {
    use defuse::core::{amounts::Amounts, intents::tokens::Transfer};
    use defuse_sandbox::{api::types::json::Base64VecU8, tx::FnCallBuilder};

    let env = Env::builder().deployer_as_super_admin().build().await;

    // Ensure the NFT issuer account name stays short enough to host `nft_test.<user>`
    // subaccounts; randomly generated names occasionally exceed the NEAR 64-char limit.
    let (user, intent_receiver) = futures::join!(
        env.create_named_user("nft_transfer_sender"),
        env.create_user()
    );

    let receiver = env
        .deploy_sub_contract(
            "receiver_stub",
            NearToken::from_near(100),
            MT_RECEIVER_STUB_WASM.to_vec(),
            None::<FnCallBuilder>,
        )
        .await
        .unwrap();

    env.tx(user.id())
        .transfer(NearToken::from_near(100))
        .await
        .unwrap();

    let nft_issuer_contract = user
        .deploy_vanilla_nft_issuer(
            "nft_test",
            user.id(),
            NFTContractMetadata {
                reference: Some("http://test.com/".to_string()),
                reference_hash: Some(Base64VecU8(DUMMY_REFERENCE_HASH.to_vec())),
                spec: NFT_METADATA_SPEC.to_string(),
                name: "Test NFT".to_string(),
                symbol: "TNFT".to_string(),
                icon: None,
                base_uri: None,
            },
        )
        .await
        .unwrap();

    let nft: Token = user
        .nft_mint(
            nft_issuer_contract.id(),
            &DUMMY_NFT1_ID.to_string(),
            user.id(),
            &TokenMetadata::default(),
        )
        .await
        .unwrap();

    assert_eq!(nft.owner_id, *user.id());

    let nft_token_id = DefuseTokenId::from(Nep171TokenId::new(
        nft_issuer_contract.id().clone(),
        DUMMY_NFT1_ID.to_string(),
    ));

    let intents = if expectation.intent_transfer {
        vec![
            receiver
                .sign_defuse_payload_default(
                    &env.defuse,
                    [Transfer {
                        receiver_id: intent_receiver.id().clone(),
                        tokens: Amounts::new(std::iter::once((nft_token_id.clone(), 1)).collect()),
                        memo: None,
                        notification: None,
                    }],
                )
                .await
                .unwrap(),
        ]
    } else {
        vec![]
    };

    let deposit_message = if intents.is_empty() {
        use defuse::core::intents::tokens::NotifyOnTransfer;

        DepositMessage {
            receiver_id: receiver.id().clone(),
            action: Some(DepositAction::Notify(NotifyOnTransfer::new(
                near_sdk::serde_json::to_string(&expectation.action).unwrap(),
            ))),
        }
    } else {
        DepositMessage {
            receiver_id: receiver.id().clone(),
            action: Some(DepositAction::Execute(ExecuteIntents {
                execute_intents: intents,
                refund_if_fails: expectation.refund_if_fails,
            })),
        }
    };

    user.nft_transfer_call(
        nft_issuer_contract.id(),
        env.defuse.id(),
        nft.token_id.clone(),
        None,
        near_sdk::serde_json::to_string(&deposit_message).unwrap(),
    )
    .await
    .unwrap();

    // Check ownership on the NFT contract
    let nft_owner = nft_issuer_contract
        .nft_token(&nft.token_id)
        .await
        .unwrap()
        .unwrap()
        .owner_id;

    if expectation.expected_sender_owns_nft {
        assert_eq!(nft_owner, *user.id(), "NFT should be owned by sender");
    } else {
        assert_eq!(
            nft_owner,
            *env.defuse.id(),
            "NFT should be owned by defuse contract"
        );
    }

    // Check if receiver owns the NFT in MT balance
    let receiver_mt_balance = env
        .defuse
        .mt_balance_of(receiver.id(), &nft_token_id.to_string())
        .await
        .unwrap();

    if expectation.expected_receiver_owns_nft {
        assert_eq!(
            receiver_mt_balance, 1,
            "Receiver should own the NFT (MT balance = 1)"
        );
    } else {
        assert_eq!(
            receiver_mt_balance, 0,
            "Receiver should not own the NFT (MT balance = 0)"
        );
    }
}
