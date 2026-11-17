//! Strict wallet input adapters for shared protocol types.
//!
//! Deserialize directly from the input so duplicate keys are not lost through
//! an intermediate JSON value. Remote derives keep the public shared types and
//! their serialization unchanged; only wallet input validation is stricter.

use std::collections::BTreeMap;

use defuse_near_promise::{
    Gas, GlobalContractId, NearPromise, NearToken, StateInit, StateInitV1,
    actions::{DeterministicStateInit, FunctionCall, NearAction, Transfer},
};
use defuse_nep641::OffchainMessage;
use serde::{Deserialize, Deserializer};
use serde_with::{DisplayFromStr, MapPreventDuplicates, base64::Base64, serde_as};

use crate::{AccountId, Timestamp};

#[serde_as]
#[derive(Deserialize)]
#[serde(remote = "NearPromise", deny_unknown_fields)]
struct Promise {
    receiver_id: AccountId,
    #[serde(default)]
    refund_to: Option<AccountId>,
    #[serde_as(as = "Vec<Action>")]
    actions: Vec<NearAction>,
}

#[derive(Deserialize)]
#[serde(
    remote = "NearAction",
    tag = "action",
    content = "payload",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum Action {
    FunctionCall(#[serde(deserialize_with = "Call::deserialize")] FunctionCall),
    Transfer(#[serde(deserialize_with = "Send::deserialize")] Transfer),
    DeterministicStateInit(
        #[serde(deserialize_with = "Initialize::deserialize")] DeterministicStateInit,
    ),
}

#[serde_as]
#[derive(Deserialize)]
#[serde(remote = "FunctionCall", deny_unknown_fields)]
struct Call {
    function_name: String,
    #[serde_as(as = "Base64")]
    #[serde(default)]
    args: Vec<u8>,
    #[serde(default)]
    deposit: NearToken,
    #[serde(default)]
    gas: Gas,
    #[serde_as(as = "DisplayFromStr")]
    #[serde(default = "default_gas_weight")]
    gas_weight: u64,
}

const fn default_gas_weight() -> u64 {
    1
}

#[derive(Deserialize)]
#[serde(remote = "Transfer", deny_unknown_fields)]
struct Send {
    amount: NearToken,
}

#[derive(Deserialize)]
#[serde(remote = "DeterministicStateInit", deny_unknown_fields)]
struct Initialize {
    #[serde(deserialize_with = "state_init")]
    state_init: StateInit,
    #[serde(default)]
    deposit: NearToken,
}

#[serde_as]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct V1 {
    code: GlobalContractId,
    #[serde_as(as = "MapPreventDuplicates<Base64, Base64>")]
    data: BTreeMap<Vec<u8>, Vec<u8>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum Init {
    V1(V1),
}

fn state_init<'de, D: Deserializer<'de>>(deserializer: D) -> Result<StateInit, D::Error> {
    let Init::V1(V1 { code, data }) = Init::deserialize(deserializer)?;
    Ok(StateInit::V1(StateInitV1 { code, data }))
}

// `serde_with` adapters let a sequence use a remote derive without changing
// `Request.external` or the shared `NearPromise`/`NearAction` types.
impl<'de> serde_with::DeserializeAs<'de, NearAction> for Action {
    fn deserialize_as<D: Deserializer<'de>>(deserializer: D) -> Result<NearAction, D::Error> {
        Self::deserialize(deserializer)
    }
}

impl<'de> serde_with::DeserializeAs<'de, NearPromise> for Promise {
    fn deserialize_as<D: Deserializer<'de>>(deserializer: D) -> Result<NearPromise, D::Error> {
        Self::deserialize(deserializer)
    }
}

pub fn promises<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<NearPromise>, D::Error> {
    serde_with::As::<Vec<Promise>>::deserialize(deserializer)
}

#[derive(Deserialize)]
#[serde(remote = "OffchainMessage", deny_unknown_fields)]
struct Offchain {
    chain_id: String,
    signer_id: AccountId,
    #[serde(default)]
    path: Vec<AccountId>,
    timestamp: Timestamp,
    payload: String,
}

pub fn offchain<'de, D: Deserializer<'de>>(deserializer: D) -> Result<OffchainMessage, D::Error> {
    Offchain::deserialize(deserializer)
}

#[cfg(all(test, feature = "json"))]
mod tests {
    use super::*;
    use crate::{Request, WalletAuthorization};

    #[test]
    fn shared_types_keep_their_permissive_json_behavior() {
        for action in [
            r#"{"action":"transfer","payload":{"amount":"1","unknown":true},"unknown":true}"#,
            r#"{"action":"function_call","payload":{"function_name":"f","unknown":true}}"#,
            r#"{"action":"deterministic_state_init","payload":{"state_init":{"V1":{"code":{"account_id":"code.near"},"data":{},"unknown":true}},"unknown":true}}"#,
        ] {
            let promise = format!(r#"{{"receiver_id":"a.near","actions":[{action}]}}"#);
            let _ = serde_json::from_str::<NearPromise>(&promise).unwrap();
            let request = format!(r#"{{"external":[{promise}]}}"#);
            assert!(serde_json::from_str::<Request>(&request).is_err());
        }
        let promise = r#"{"receiver_id":"a.near","actions":[],"unknown":true}"#;
        let _ = serde_json::from_str::<NearPromise>(promise).unwrap();
        assert!(
            serde_json::from_str::<Request>(&format!(r#"{{"external":[{promise}]}}"#)).is_err()
        );

        let msg = r#"{"chain_id":"test","signer_id":"a.near","timestamp":"1970-01-01T00:00:00Z","payload":"x","unknown":true}"#;
        serde_json::from_str::<OffchainMessage>(msg).unwrap();
        let authorization = format!(r#"{{"signature":{{"msg":{msg},"proof":""}}}}"#);
        assert!(serde_json::from_str::<WalletAuthorization>(&authorization).is_err());
    }

    #[test]
    fn duplicate_fields_are_rejected_before_conversion() {
        for promise in [
            r#"{"receiver_id":"a.near","receiver_id":"b.near","actions":[]}"#,
            r#"{"receiver_id":"a.near","actions":[{"action":"transfer","action":"transfer","payload":{"amount":"1"}}]}"#,
            r#"{"receiver_id":"a.near","actions":[{"action":"transfer","payload":{"amount":"1","amount":"2"}}]}"#,
            r#"{"receiver_id":"a.near","actions":[{"action":"function_call","payload":{"function_name":"f","function_name":"g"}}]}"#,
            r#"{"receiver_id":"a.near","actions":[{"action":"deterministic_state_init","payload":{"state_init":{"V1":{"code":{"account_id":"code.near"},"data":{"AA==":"AQ==","AA==":"Ag=="}}}}}]}"#,
            r#"{"receiver_id":"a.near","actions":[{"action":"deterministic_state_init","payload":{"state_init":{"V1":{"code":{"account_id":"code.near"},"data":{"AA==":"AQ==","AA":"Ag=="}}}}}]}"#,
        ] {
            let request = format!(r#"{{"external":[{promise}]}}"#);
            assert!(
                serde_json::from_str::<Request>(&request).is_err(),
                "{request}"
            );
        }
        let authorization = r#"{"signature":{"msg":{"chain_id":"test","signer_id":"a.near","timestamp":"1970-01-01T00:00:00Z","payload":"x","payload":"y"},"proof":""}}"#;
        assert!(serde_json::from_str::<WalletAuthorization>(authorization).is_err());
    }

    #[test]
    fn adapters_preserve_defaults_serialization_and_opaque_payloads() {
        let promise = r#"{"receiver_id":"a.near","refund_to":null,"actions":[{"action":"function_call","payload":{"function_name":"f"}},{"action":"transfer","payload":{"amount":"1"}},{"action":"deterministic_state_init","payload":{"state_init":{"V1":{"code":{"account_id":"code.near"},"data":{"AA==":"AQ=="}}}}}]}"#;
        let shared: NearPromise = serde_json::from_str(promise).unwrap();
        let request: Request =
            serde_json::from_str(&format!(r#"{{"external":[{promise}]}}"#)).unwrap();
        assert_eq!(request.external.as_slice(), std::slice::from_ref(&shared));
        assert_eq!(
            serde_json::to_vec(&request.external[0]).unwrap(),
            serde_json::to_vec(&shared).unwrap()
        );
        let msg = r#"{"chain_id":"test","signer_id":"a.near","timestamp":"1970-01-01T00:00:00Z","payload":"{ \"x\":1,\"x\":2 }"}"#;
        let shared: OffchainMessage = serde_json::from_str(msg).unwrap();
        let authorization = format!(r#"{{"signature":{{"msg":{msg},"proof":""}}}}"#);
        let WalletAuthorization::Signature { msg, .. } =
            serde_json::from_str(&authorization).unwrap()
        else {
            panic!("signature");
        };
        assert_eq!(msg, shared);
        assert_eq!(
            serde_json::to_vec(&msg).unwrap(),
            serde_json::to_vec(&shared).unwrap()
        );
        assert_eq!(msg.payload, "{ \"x\":1,\"x\":2 }");
    }
}
