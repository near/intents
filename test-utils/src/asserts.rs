use std::{cell::LazyCell, fmt::Display};

const DISABLE_STRING_CHECKS_ENV_VAR: &str = "DEFUSE_SKIP_STRING_ERROR_CHECKS";

thread_local! {
    static DISABLE_STRING_CHECKS: LazyCell<bool> = LazyCell::new(
        || std::env::var(DISABLE_STRING_CHECKS_ENV_VAR).is_ok(),
    );
}

pub trait ResultAssertsExt {
    fn assert_err_contains(&self, to_contain: impl AsRef<str>);
}

impl<T, E> ResultAssertsExt for Result<T, E>
where
    E: Display,
{
    #[track_caller]
    fn assert_err_contains(&self, to_contain: impl AsRef<str>) {
        let to_contain = to_contain.as_ref();
        match self {
            Ok(_) => panic!("Result::unwrap_err() on Result::Ok()"),
            Err(e) => {
                // Define the env var to check strings in errors
                let check_string = !DISABLE_STRING_CHECKS.with(|b| **b);
                if check_string {
                    let error_string = e.to_string();
                    assert!(
                        e.to_string().contains(to_contain),
                        "Result::unwrap_err() successful, but the error string does not contain the expected string.\nError string: `{error_string}`\nshould have contained: `{to_contain}`"
                    );
                } else {
                    eprintln!(
                        "WARNING: Ignoring string contents' checks in errors due to env var `{DISABLE_STRING_CHECKS_ENV_VAR}` being defined"
                    );
                }
            }
        }
    }
}
