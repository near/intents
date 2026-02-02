use crate::extensions::escrow::contract::{
    ContractStorage, Deadline, OverrideSend, Params,
    token_id::{TokenId, nep141::Nep141TokenId},
};
use rstest::rstest;
use std::time::Duration;

use crate::tests::escrow::helpers::{Env, env};

#[rstest]
#[tokio::test]
async fn init_state_has_single_empty_key(#[future(awt)] env: Env) {
    let src_token: TokenId = Nep141TokenId::new(env.src_ft.id().clone()).into();
    let dst_token: TokenId = Nep141TokenId::new(env.dst_ft.id().clone()).into();

    let params = Params {
        maker: env.maker.id().clone(),
        src_token,
        dst_token,
        price: "1".parse().unwrap(),
        deadline: Deadline::timeout(Duration::from_secs(60)),
        partial_fills_allowed: false,
        refund_src_to: OverrideSend::default(),
        receive_dst_to: OverrideSend::default(),
        taker_whitelist: Default::default(),
        protocol_fees: None,
        integrator_fees: Default::default(),
        auth_caller: None,
        salt: [0; 32],
    };

    let state_map = ContractStorage::init_state(&params).unwrap();

    assert_eq!(state_map.len(), 1, "should have exactly 1 key/value pair");

    for (key, value) in &state_map {
        println!("key length: {}, key: {:?}", key.len(), key);
        println!("value length: {}, value: {:?}", value.len(), value);
    }

    let key = state_map.keys().next().unwrap();
    assert_eq!(key.len(), 0, "key should be empty (STATE_KEY = b\"\")");
}
