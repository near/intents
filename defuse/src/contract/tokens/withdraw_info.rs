use defuse_core::intents::tokens::{FtWithdraw, MtWithdraw, NftWithdraw};
use near_sdk::{AccountId, near};

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub enum WithdrawInfo {
    NEP141 {
        receiver_id: AccountId,
        memo: Option<String>,
        msg: Option<String>,
    },
    NEP171 {
        receiver_id: AccountId,
        memo: Option<String>,
        msg: Option<String>,
    },
    NEP245 {
        receiver_id: AccountId,
        memo: Option<String>,
        msg: Option<String>,
    },
}

impl From<FtWithdraw> for WithdrawInfo {
    fn from(value: FtWithdraw) -> Self {
        Self::NEP141 {
            receiver_id: value.receiver_id,
            memo: value.memo,
            msg: value.msg,
        }
    }
}

impl From<NftWithdraw> for WithdrawInfo {
    fn from(value: NftWithdraw) -> Self {
        Self::NEP171 {
            receiver_id: value.receiver_id,
            memo: value.memo,
            msg: value.msg,
        }
    }
}

impl From<MtWithdraw> for WithdrawInfo {
    fn from(value: MtWithdraw) -> Self {
        Self::NEP171 {
            receiver_id: value.receiver_id,
            memo: value.memo,
            msg: value.msg,
        }
    }
}
