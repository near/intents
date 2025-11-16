use near_sdk::near;

use crate::Params;

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct Message {
    pub params: Params,
    pub action: Action,
}

#[near(serializers = [json])]
#[serde(tag = "action", content = "data", rename_all = "snake_case")]
#[derive(Debug, Clone)]
pub enum Action {
    Close(Params),
}
