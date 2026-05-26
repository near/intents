use defuse_outlayer_proto as proto;
use near_account_id::AccountId;

use crate::AppId;

impl TryFrom<proto::AppId> for AppId<'static> {
    type Error = String;

    fn try_from(p: proto::AppId) -> Result<Self, Self::Error> {
        match p
            .variant
            .ok_or_else(|| "missing AppId variant".to_owned())?
        {
            proto::app_id::Variant::Near(s) => s
                .parse::<AccountId>()
                .map(Self::from)
                .map_err(|e| e.to_string()),
        }
    }
}
