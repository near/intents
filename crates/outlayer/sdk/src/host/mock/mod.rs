mod config;

use std::cell::RefCell;

use defuse_outlayer_host::State;

use self::config::MockConfig;

thread_local! {
    pub(crate) static HOST: RefCell<State<'static>> =
        RefCell::new(
            MockConfig::from_env()
                .inspect_err(|err|
                    eprintln!("WARN: unable to initialize test config, fallback to default: {err:#}")
                )
                .unwrap_or_default()
                .build()
        );
}

// TODO: functions to set/modify mock
