use defuse_sandbox::extensions::wallet::sdk::MAINNET;
use defuse_sandbox::extensions::wallet::sdk::{
    AccountId, AuthErrorKind, AuthMessage, AuthSignerBinding, AuthorizationResolution,
    SignedAuthMessage, Timestamp, WalletBuilder,
};
use defuse_test_utils::wasms::WALLET_NO_SIGN_WASM;
use defuse_wallet_relayer::wallet::client::WResolveAuthArgs;
use std::{borrow::Cow, collections::BTreeSet, time::Duration};

use super::*;

const PURPOSE: &str = "PROVE_OWNERSHIP";
const RECIPIENT: &str = "trezu.app";
const PAYLOAD: &str = "Login to trezu.app at 2026-07-16T00:00:00Z";

impl Env {
    /// The by-account factory id of `wallet_global_id`, for building the
    /// `allowed_factory_ids` allow-list of `Code` bindings. Panics for a
    /// code-hash deployment (which `Code` bindings can't target).
    fn factory_id(&self) -> AccountId {
        match &self.wallet_global_id {
            GlobalContractId::AccountId(id) => id.clone(),
            other @ GlobalContractId::CodeHash(_) => {
                panic!("factory is not deployed by account id: {other:?}")
            }
        }
    }

    /// Materialize the wallet's deterministic account on-chain, so that
    /// `w_resolve_auth` view calls can be made against it.
    async fn materialize(
        &self,
        wallet: &Wallet<WalletEd25519, WalletEd25519Signer<ed25519_dalek::SigningKey>>,
    ) {
        self.transaction(wallet.account_id().clone())
            .state_init(wallet.deterministic_state_init().clone(), NearToken::ZERO)
            .await
            .unwrap()
            .result()
            .unwrap();
    }

    async fn resolve_auth(
        &self,
        account_id: &AccountId,
        purpose: &str,
        recipient: &str,
        signed: &SignedAuthMessage,
    ) -> AuthorizationResolution {
        self.contract::<WalletContract>(account_id)
            .w_resolve_auth(WResolveAuthArgs {
                purpose: Cow::Borrowed(purpose),
                recipient: Cow::Borrowed(recipient),
                authorization: Cow::Owned(serde_json::to_string(signed).unwrap()),
            })
            .finality(Optimistic)
            .await
            .unwrap()
    }
}

fn assert_invalid(resolution: &AuthorizationResolution, expected_kind: AuthErrorKind) {
    let AuthorizationResolution::Invalid { error_kind, .. } = resolution else {
        panic!("expected INVALID, got: {resolution:?}");
    };
    assert_eq!(*error_kind, expected_kind);
}

#[rstest]
#[awt]
#[tokio::test]
async fn test_resolve_auth_signer_id(#[future] env: Env) {
    let wallet = env.generate_wallet();
    env.materialize(&wallet).await;

    let signed = wallet
        .sign_auth(wallet.auth_message(PURPOSE, RECIPIENT, PAYLOAD))
        .await
        .unwrap();

    assert_eq!(
        env.resolve_auth(wallet.account_id(), PURPOSE, RECIPIENT, &signed)
            .await,
        AuthorizationResolution::Resolved {
            payload: PAYLOAD.to_string(),
        },
    );

    // purpose binding
    assert_invalid(
        &env.resolve_auth(
            wallet.account_id(),
            "trezu/proposal:VoteApprove",
            RECIPIENT,
            &signed,
        )
        .await,
        AuthErrorKind::InvalidInput,
    );

    // recipient binding
    assert_invalid(
        &env.resolve_auth(wallet.account_id(), PURPOSE, "evil.app", &signed)
            .await,
        AuthErrorKind::InvalidInput,
    );

    // malformed blob
    assert_invalid(
        &env.contract::<WalletContract>(wallet.account_id())
            .w_resolve_auth(WResolveAuthArgs {
                purpose: Cow::Borrowed(PURPOSE),
                recipient: Cow::Borrowed(RECIPIENT),
                authorization: Cow::Borrowed("not json"),
            })
            .finality(Optimistic)
            .await
            .unwrap(),
        AuthErrorKind::InvalidInput,
    );

    // tampered signature
    let tampered = SignedAuthMessage {
        message: AuthMessage {
            payload: "another payload".to_string(),
            ..signed.message.clone()
        },
        proof: signed.proof.clone(),
    };
    assert_invalid(
        &env.resolve_auth(wallet.account_id(), PURPOSE, RECIPIENT, &tampered)
            .await,
        AuthErrorKind::InvalidSignature,
    );
}

#[rstest]
#[awt]
#[tokio::test]
async fn test_resolve_auth_code_binding(#[future] env: Env) {
    let wallet = env.generate_wallet();
    env.materialize(&wallet).await;

    // NOTE: resolution of this binding also guards the near-sdk >= 5.29.0
    // fix for `env::current_global_contract_id()` returning the global
    // contract's account id (not the wallet's own account id) for
    // GlobalByAccount deployments: the contract reconstructs the StateInit
    // from the code it is currently running under.
    let msg = wallet.auth_message_code_binding([env.factory_id()], PURPOSE, RECIPIENT, PAYLOAD);
    assert_eq!(
        msg.signer,
        AuthSignerBinding::Code {
            allowed_factory_ids: BTreeSet::from([env.factory_id()]),
            signature_enabled: true,
            subwallet_id: 0,
            timeout: wallet.timeout(),
            extensions: BTreeSet::new(),
        },
    );

    let signed = wallet.sign_auth(msg).await.unwrap();

    assert_eq!(
        env.resolve_auth(wallet.account_id(), PURPOSE, RECIPIENT, &signed)
            .await,
        AuthorizationResolution::Resolved {
            payload: PAYLOAD.to_string(),
        },
    );

    // mutate live config: add an extension
    let extension = env.create_subaccount("extension", NearToken::ZERO).await;
    let (msg, proof) = wallet
        .sign(Request::new().internal([WalletOp::AddExtension {
            account_id: extension.account_id().clone(),
        }]))
        .await
        .unwrap();
    assert!(
        env.relayer
            .w_execute_signed(
                defuse_wallet_relayer::WalletRelayRequest::new(msg, proof),
                NearToken::from_yoctonear(1),
                None,
            )
            .await
            .unwrap()
            .is_success()
    );

    // the binding commits to the INITIAL state (which determines the
    // account id), so config mutations do NOT invalidate it: the same
    // initial-defaults envelope still resolves
    let signed = wallet
        .sign_auth(wallet.auth_message_code_binding(
            [env.factory_id()],
            PURPOSE,
            RECIPIENT,
            PAYLOAD,
        ))
        .await
        .unwrap();
    assert_eq!(
        env.resolve_auth(wallet.account_id(), PURPOSE, RECIPIENT, &signed)
            .await,
        AuthorizationResolution::Resolved {
            payload: PAYLOAD.to_string(),
        },
    );

    // ...while an envelope built from the MUTATED (live) config derives a
    // different account id and MUST be rejected
    let mut mutated_msg =
        wallet.auth_message_code_binding([env.factory_id()], PURPOSE, RECIPIENT, PAYLOAD);
    let AuthSignerBinding::Code { extensions, .. } = &mut mutated_msg.signer else {
        unreachable!()
    };
    extensions.insert(extension.account_id().clone());
    let mutated_signed = wallet.sign_auth(mutated_msg).await.unwrap();
    assert_invalid(
        &env.resolve_auth(wallet.account_id(), PURPOSE, RECIPIENT, &mutated_signed)
            .await,
        AuthErrorKind::InvalidInput,
    );

    // SignerId binding is unaffected by config mutation
    let signer_id_signed = wallet
        .sign_auth(wallet.auth_message(PURPOSE, RECIPIENT, PAYLOAD))
        .await
        .unwrap();
    assert_eq!(
        env.resolve_auth(wallet.account_id(), PURPOSE, RECIPIENT, &signer_id_signed)
            .await,
        AuthorizationResolution::Resolved {
            payload: PAYLOAD.to_string(),
        },
    );
}

#[rstest]
#[awt]
#[tokio::test]
async fn test_resolve_auth_subwallet_isolation(#[future] env: Env) {
    let key = ed25519_dalek::SigningKey::generate(&mut rng());

    let wallet0 = Wallet::<WalletEd25519, _>::new(
        env.wallet_global_id.clone(),
        WalletEd25519Signer(key.clone()),
    );
    let wallet1: Wallet<WalletEd25519, _> = WalletBuilder::new()
        .subwallet_id(1)
        .build(env.wallet_global_id.clone(), WalletEd25519Signer(key));
    assert_ne!(wallet0.account_id(), wallet1.account_id());

    env.materialize(&wallet0).await;
    env.materialize(&wallet1).await;

    // both bindings for wallet0 MUST NOT resolve on wallet1,
    // even though it's controlled by the same key
    let signer_id_signed = wallet0
        .sign_auth(wallet0.auth_message(PURPOSE, RECIPIENT, PAYLOAD))
        .await
        .unwrap();
    let code_signed = wallet0
        .sign_auth(wallet0.auth_message_code_binding(
            [env.factory_id()],
            PURPOSE,
            RECIPIENT,
            PAYLOAD,
        ))
        .await
        .unwrap();

    for signed in [&signer_id_signed, &code_signed] {
        assert_eq!(
            env.resolve_auth(wallet0.account_id(), PURPOSE, RECIPIENT, signed)
                .await,
            AuthorizationResolution::Resolved {
                payload: PAYLOAD.to_string(),
            },
        );
        assert_invalid(
            &env.resolve_auth(wallet1.account_id(), PURPOSE, RECIPIENT, signed)
                .await,
            AuthErrorKind::InvalidInput,
        );
    }
}

#[rstest]
#[awt]
#[tokio::test]
async fn test_resolve_auth_signature_disabled(#[future] env: Env) {
    let wallet = env.generate_wallet();
    env.materialize(&wallet).await;

    let extension = env.create_subaccount("extension", NearToken::ZERO).await;
    let (msg, proof) = wallet
        .sign(Request::new().internal([
            WalletOp::AddExtension {
                account_id: extension.account_id().clone(),
            },
            WalletOp::SetSignatureMode { enable: false },
        ]))
        .await
        .unwrap();
    assert!(
        env.relayer
            .w_execute_signed(
                defuse_wallet_relayer::WalletRelayRequest::new(msg, proof),
                NearToken::from_yoctonear(1),
                None,
            )
            .await
            .unwrap()
            .is_success()
    );

    let signed = wallet
        .sign_auth(wallet.auth_message(PURPOSE, RECIPIENT, PAYLOAD))
        .await
        .unwrap();
    assert_invalid(
        &env.resolve_auth(wallet.account_id(), PURPOSE, RECIPIENT, &signed)
            .await,
        AuthErrorKind::InvalidInput,
    );
}

#[rstest]
#[awt]
#[tokio::test]
async fn test_resolve_auth_validity_window(#[future] env: Env) {
    let wallet = env.generate_wallet();
    env.materialize(&wallet).await;

    // expired
    let mut msg = wallet.auth_message(PURPOSE, RECIPIENT, PAYLOAD);
    msg.created_at = Timestamp::now() - Duration::from_hours(3);
    let signed = wallet.sign_auth(msg).await.unwrap();
    assert_invalid(
        &env.resolve_auth(wallet.account_id(), PURPOSE, RECIPIENT, &signed)
            .await,
        AuthErrorKind::InvalidInput,
    );

    // from the future
    let mut msg = wallet.auth_message(PURPOSE, RECIPIENT, PAYLOAD);
    msg.created_at = Timestamp::now() + Duration::from_hours(1);
    let signed = wallet.sign_auth(msg).await.unwrap();
    assert_invalid(
        &env.resolve_auth(wallet.account_id(), PURPOSE, RECIPIENT, &signed)
            .await,
        AuthErrorKind::InvalidInput,
    );
}

#[rstest]
#[awt]
#[tokio::test]
async fn test_resolve_auth_code_binding_rejects_code_hash(#[future] root: Near) {
    // A wallet whose code is deployed immutably (by code hash) has no factory
    // account id, so it can never appear in `allowed_factory_ids`. A `Code`
    // binding therefore cannot target it — such wallets must use `SignerId`.
    let global_id = root
        .deploy_immutable_global_contract(
            root.account_id().sub_account("wallet-hash").unwrap(),
            defuse_test_utils::wasms::WALLET_ED25519_WASM.clone(),
            NearToken::from_near(1000),
        )
        .await
        .unwrap();
    assert!(matches!(global_id, GlobalContractId::CodeHash(_)));

    let env = Env {
        wallet_global_id: global_id,
        relayer: WalletRelayer::new(root.clone()),
        root,
    };

    let wallet = env.generate_wallet();
    env.materialize(&wallet).await;

    // Rejected regardless of allow-list contents: the running code is a
    // CodeHash deployment, not a by-account factory.
    let placeholder: AccountId = "any-factory.near".parse().unwrap();
    let code_signed = wallet
        .sign_auth(wallet.auth_message_code_binding([placeholder], PURPOSE, RECIPIENT, PAYLOAD))
        .await
        .unwrap();
    assert_invalid(
        &env.resolve_auth(wallet.account_id(), PURPOSE, RECIPIENT, &code_signed)
            .await,
        AuthErrorKind::InvalidInput,
    );

    // ...but `SignerId` binding still works for code-hash deployments.
    let signer_id_signed = wallet
        .sign_auth(wallet.auth_message(PURPOSE, RECIPIENT, PAYLOAD))
        .await
        .unwrap();
    assert_eq!(
        env.resolve_auth(wallet.account_id(), PURPOSE, RECIPIENT, &signer_id_signed)
            .await,
        AuthorizationResolution::Resolved {
            payload: PAYLOAD.to_string(),
        },
    );
}

#[rstest]
#[awt]
#[tokio::test]
async fn test_resolve_auth_factory_allow_list(#[future] env: Env) {
    // Two by-account factories of the SAME curve, same key + config → sibling
    // accounts of one key holder. `allowed_factory_ids` is what stops a single
    // signed `Code`-binding message from resolving against more than one.
    let wallet = env.generate_wallet();
    env.materialize(&wallet).await;

    let sibling_factory = env
        .deploy_upgradable_global_contract(
            env.account_id().sub_account("wallet-sibling").unwrap(),
            defuse_test_utils::wasms::WALLET_ED25519_WASM.clone(),
            NearToken::from_near(1000),
        )
        .await
        .unwrap();
    let GlobalContractId::AccountId(sibling_factory_id) = sibling_factory.clone() else {
        unreachable!()
    };
    let sibling = Wallet::<WalletEd25519, _>::new(
        sibling_factory,
        WalletEd25519Signer(wallet.signer().as_ref().clone()),
    );
    assert_ne!(wallet.account_id(), sibling.account_id());
    env.materialize(&sibling).await;

    // Listing ONLY the canonical factory: resolves on the canonical account,
    // REJECTED on the sibling (its factory isn't allow-listed) — the
    // sibling-account replay is closed.
    let canonical_only = wallet
        .sign_auth(wallet.auth_message_code_binding(
            [env.factory_id()],
            PURPOSE,
            RECIPIENT,
            PAYLOAD,
        ))
        .await
        .unwrap();
    assert_eq!(
        env.resolve_auth(wallet.account_id(), PURPOSE, RECIPIENT, &canonical_only)
            .await,
        AuthorizationResolution::Resolved {
            payload: PAYLOAD.to_string(),
        },
    );
    assert_invalid(
        &env.resolve_auth(sibling.account_id(), PURPOSE, RECIPIENT, &canonical_only)
            .await,
        AuthErrorKind::InvalidInput,
    );

    // Listing TWO same-curve factories violates the documented invariant: the
    // SAME signed message now resolves against BOTH accounts (cross-account
    // replay). Pinned here so the invariant is never relaxed by accident.
    let both = wallet
        .sign_auth(wallet.auth_message_code_binding(
            [env.factory_id(), sibling_factory_id],
            PURPOSE,
            RECIPIENT,
            PAYLOAD,
        ))
        .await
        .unwrap();
    for account in [wallet.account_id(), sibling.account_id()] {
        assert_eq!(
            env.resolve_auth(account, PURPOSE, RECIPIENT, &both).await,
            AuthorizationResolution::Resolved {
                payload: PAYLOAD.to_string(),
            },
        );
    }
}

#[rstest]
#[awt]
#[tokio::test]
async fn test_resolve_auth_no_sign(
    #[future]
    #[with(WALLET_NO_SIGN_WASM.clone())]
    env: Env,
) {
    use defuse_sandbox::kit::{self, Action, Final, UseGlobalContractAction};

    let user = env
        .create_subaccount("user", NearToken::from_near(10))
        .await;

    // initialize no-sign wallet contract on existing account
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

    // no-sign wallets have no signing identity: any authorization
    // MUST be rejected with INVALID_SIGNATURE
    let signed = SignedAuthMessage {
        message: AuthMessage {
            chain_id: MAINNET.to_string(),
            signer: AuthSignerBinding::SignerId {
                signer_id: user.account_id().clone(),
            },
            purpose: PURPOSE.to_string(),
            recipient: RECIPIENT.to_string(),
            payload: PAYLOAD.to_string(),
            created_at: Timestamp::now(),
            timeout: Duration::from_hours(1),
        },
        proof: String::new(),
    };
    assert_invalid(
        &env.resolve_auth(user.account_id(), PURPOSE, RECIPIENT, &signed)
            .await,
        AuthErrorKind::InvalidSignature,
    );
}
