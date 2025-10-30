use near_sdk::{AccountIdRef, Gas, NearToken, json_types::U128, serde_json::json};

use crate::env::SigningAccount;

impl SigningAccount {
    pub async fn mt_transfer_call(
        &self,
        mt: &AccountIdRef,
        receiver_id: &AccountIdRef,
        token_id: impl AsRef<str>,
        amount: u128,
        msg: impl AsRef<str>,
    ) -> u128 {
        let args = json!({
            "receiver_id": receiver_id,
            "token_id": token_id.as_ref(),
            "amount": U128(amount),
            "msg": msg.as_ref(),
        });

        let [sent] = self
            .tx(mt.to_owned())
            .function_call_json(
                "mt_transfer_call",
                args.clone(),
                Gas::from_tgas(300),
                NearToken::from_yoctonear(1),
            )
            .await
            .unwrap()
            .into_result()
            .inspect(|r| println!("{mt}::mt_transfer_call({}) -> {:#?}", args, r.logs()))
            .unwrap()
            .json::<Vec<U128>>()
            .expect("JSON")
            .try_into()
            .expect("sent more than one token");
        sent.0
    }
}
