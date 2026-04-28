use std::{borrow::Cow, cell::RefCell};

use defuse_outlayer_host::{Context, InMemorySigner, State};
use defuse_outlayer_primitives::{AccountIdRef, AppId};

thread_local! {
    pub(crate) static HOST: RefCell<State<'static>> =
        RefCell::new(State::new(
            Context {
                app_id: TEST_APP_ID,
            },
            Cow::Owned(InMemorySigner::from_seed(TEST_SEED)),
        ));
}

// Generated via near-cli@0.26.0:
// ```sh
// near contract state-init \
//   use-global-account-id 'test' \
//   data-from-json "$(near oa -q \
//       --admin-id 'test' \
//       --code-hash '0000000000000000000000000000000000000000000000000000000000000000' \
//       --code-url 'data:' \
//   )" inspect account-id
// ```
const TEST_APP_ID: AppId = AppId::Near(Cow::Borrowed(AccountIdRef::new_or_panic(
    "0sab1c86e60758fe3e8fc7ae40ecd2df1a07513ca9",
)));

const TEST_SEED: &[u8] = b"test";

// TODO: functions to set/modify mock
