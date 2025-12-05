use near_sdk::{NearToken, near, state_init::StateInit};

#[near(serializers = [borsh, json])]
#[derive(Debug, Clone)]
pub struct StateInitArgs {
    pub state_init: StateInit,

    #[serde(
        rename = "state_init_amount",
        default,
        skip_serializing_if = "NearToken::is_zero"
    )]
    pub amount: NearToken,
}
