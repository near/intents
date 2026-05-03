use std::{borrow::Cow, cell::RefCell};

use defuse_outlayer_host::{Context, InMemorySigner, State};
use defuse_outlayer_primitives::{AccountIdRef, AppId};

thread_local! {
    pub(crate) static HOST: RefCell<State> =
        RefCell::new(State::new(
            Context {
                app_id: TEST_APP_ID,
            },
            InMemorySigner::from_seed(TEST_SEED),
        ));
}

// Generated via near-cli@0.26.1:
// ```sh
// near contract state-init \
//   use-global-account-id 'test' \
//   data-from-json "$(near oa -q \
//       --admin-id 'test' \
//       --code-hash '0000000000000000000000000000000000000000000000000000000000000000' \
//       --code-url 'data:application/wasm;base64,' \
//   )" inspect account-id
// ```
const TEST_APP_ID: AppId = AppId::Near(Cow::Borrowed(AccountIdRef::new_or_panic(
    "0se1573c9dff58d4a57384dee048c9b1a809fb6839",
)));

const TEST_SEED: &[u8] = b"test";

// TODO: functions to set/modify mock
