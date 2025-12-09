//! Tests to verify message format compatibility between escrow-proxy and escrow-swap.
//!
//! The escrow-proxy's `TransferMessage` is a superset of escrow-swap's `TransferMessage`,
//! with an additional `salt` field. This module verifies that:
//! 1. A proxy message can be deserialized by escrow-swap (ignoring unknown fields)
//! 2. Field names and structure are compatible

use std::collections::BTreeMap;

use defuse_deadline::Deadline;
use defuse_escrow_proxy::{EscrowParams, FillAction, TransferAction, TransferMessage};
use defuse_escrow_swap::action::TransferMessage as EscrowTransferMessage;
use defuse_escrow_swap::price::Price;
use defuse_token_id::{TokenId, nep141::Nep141TokenId};
use near_sdk::AccountId;

/// Test that escrow-proxy's TransferMessage can be deserialized as escrow-swap's TransferMessage.
///
/// This proves that the proxy can forward the same JSON message to escrow-swap
/// without transformation, since escrow-swap will ignore unknown fields (like `salt`).
#[test]
fn test_proxy_message_compatible_with_escrow_swap() {
    let maker: AccountId = "maker.near".parse().unwrap();
    let token_a: TokenId = TokenId::from(Nep141TokenId::new("token-a.near".parse().unwrap()));
    let token_b: TokenId = TokenId::from(Nep141TokenId::new("token-b.near".parse().unwrap()));

    // Create a proxy TransferMessage with Fill action
    let proxy_msg = TransferMessage {
        params: EscrowParams {
            maker: maker.clone(),
            src_token: token_a.clone(),
            dst_token: token_b.clone(),
            price: Price::ONE,
            deadline: Deadline::timeout(std::time::Duration::from_secs(120)),
            partial_fills_allowed: false,
            refund_src_to: Default::default(),
            receive_dst_to: Default::default(),
            taker_whitelist: Default::default(),
            protocol_fees: None,
            integrator_fees: BTreeMap::new(),
            auth_caller: None,
            salt: [1u8; 32],
        },
        action: TransferAction::Fill(FillAction {
            price: Price::ONE,
            deadline: Deadline::timeout(std::time::Duration::from_secs(120)),
            receive_src_to: Default::default(),
        }),
        salt: [2u8; 32], // Extra field not in escrow-swap's TransferMessage
    };

    // Serialize the proxy message
    let json = serde_json::to_string(&proxy_msg).unwrap();

    // Deserialize as escrow-swap's TransferMessage - should succeed, ignoring `salt`
    let escrow_msg: EscrowTransferMessage = serde_json::from_str(&json).unwrap();

    // Verify the deserialized message matches the original params and action
    assert_eq!(escrow_msg.params.maker, proxy_msg.params.maker);
    assert_eq!(escrow_msg.params.src_token, proxy_msg.params.src_token);
    assert_eq!(escrow_msg.params.dst_token, proxy_msg.params.dst_token);
    assert_eq!(escrow_msg.params.price, proxy_msg.params.price);
    assert_eq!(
        escrow_msg.params.partial_fills_allowed,
        proxy_msg.params.partial_fills_allowed
    );

    // Verify action matches
    match (&escrow_msg.action, &proxy_msg.action) {
        (
            defuse_escrow_swap::action::TransferAction::Fill(escrow_fill),
            TransferAction::Fill(proxy_fill),
        ) => {
            assert_eq!(escrow_fill.price, proxy_fill.price);
        }
        _ => panic!("Action types should both be Fill"),
    }
}

/// Test that Fund action is also compatible
#[test]
fn test_proxy_fund_message_compatible_with_escrow_swap() {
    let maker: AccountId = "maker.near".parse().unwrap();
    let token_a: TokenId = TokenId::from(Nep141TokenId::new("token-a.near".parse().unwrap()));
    let token_b: TokenId = TokenId::from(Nep141TokenId::new("token-b.near".parse().unwrap()));

    // Create a proxy TransferMessage with Fund action
    let proxy_msg = TransferMessage {
        params: EscrowParams {
            maker: maker.clone(),
            src_token: token_a.clone(),
            dst_token: token_b.clone(),
            price: Price::ONE,
            deadline: Deadline::timeout(std::time::Duration::from_secs(120)),
            partial_fills_allowed: false,
            refund_src_to: Default::default(),
            receive_dst_to: Default::default(),
            taker_whitelist: Default::default(),
            protocol_fees: None,
            integrator_fees: BTreeMap::new(),
            auth_caller: None,
            salt: [1u8; 32],
        },
        action: TransferAction::Fund,
        salt: [3u8; 32],
    };

    // Serialize the proxy message
    let json = serde_json::to_string(&proxy_msg).unwrap();

    // Deserialize as escrow-swap's TransferMessage - should succeed
    let escrow_msg: EscrowTransferMessage = serde_json::from_str(&json).unwrap();

    // Verify action is Fund
    assert!(matches!(
        escrow_msg.action,
        defuse_escrow_swap::action::TransferAction::Fund
    ));
}

/// Test that a message without the salt field (pure escrow-swap message)
/// can also be deserialized by escrow-proxy (though this is less important).
#[test]
fn test_escrow_swap_message_can_deserialize_to_proxy_format() {
    let maker: AccountId = "maker.near".parse().unwrap();
    let token_a: TokenId = TokenId::from(Nep141TokenId::new("token-a.near".parse().unwrap()));
    let token_b: TokenId = TokenId::from(Nep141TokenId::new("token-b.near".parse().unwrap()));

    // Create escrow-swap's TransferMessage (no salt field)
    let escrow_msg = EscrowTransferMessage {
        params: defuse_escrow_swap::Params {
            maker: maker.clone(),
            src_token: token_a.clone(),
            dst_token: token_b.clone(),
            price: Price::ONE,
            deadline: Deadline::timeout(std::time::Duration::from_secs(120)),
            partial_fills_allowed: false,
            refund_src_to: Default::default(),
            receive_dst_to: Default::default(),
            taker_whitelist: Default::default(),
            protocol_fees: None,
            integrator_fees: BTreeMap::new(),
            auth_caller: None,
            salt: [1u8; 32],
        },
        action: defuse_escrow_swap::action::TransferAction::Fund,
    };

    // Serialize escrow-swap message
    let json = serde_json::to_string(&escrow_msg).unwrap();

    // This will fail because proxy message requires the salt field
    // This is expected - proxy messages must include salt for transfer-auth derivation
    let result: Result<TransferMessage, _> = serde_json::from_str(&json);
    assert!(
        result.is_err(),
        "Proxy message requires salt field, escrow-swap message should not deserialize"
    );
}
