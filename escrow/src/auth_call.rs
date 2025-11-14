use near_sdk::near;

use crate::state::Params;

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
    Close(CloseAction),
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct CloseAction {
    pub fixed_params: Params,
}
