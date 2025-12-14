use near_api::types::transaction::result::{ExecutionOutcome, ValueOrReceiptId};
use std::fmt::Debug;

use std::{fs, path::Path};

pub fn read_wasm(name: impl AsRef<Path>) -> Vec<u8> {
    let filename = fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../")
            .join(name)
            .with_extension("wasm"),
    )
    .unwrap_or_else(|e| panic!("Failed to canonicalize path: {e}"));

    println!("Reading WASM file at {filename:?}");

    fs::read(&filename).unwrap_or_else(|e| panic!("Failed to read WASM file at {filename:?}: {e}"))
}

pub struct TestExecutionOutcome<'a>(&'a ExecutionOutcome);

impl Debug for TestExecutionOutcome<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: ({}) ", self.0.executor_id, self.0.gas_burnt)?;
        if !self.0.logs.is_empty() {
            f.debug_list().entries(&self.0.logs).finish()?;
        }
        match self.0.clone().into_result() {
            Ok(v) => {
                if let ValueOrReceiptId::Value(value) = v {
                    let bytes = value.raw_bytes().unwrap();
                    if !bytes.is_empty() {
                        write!(f, ", OK: {bytes:?}")?;
                    }
                }
                Ok(())
            }
            Err(err) => write!(f, ", FAIL: {err:#?}"),
        }
    }
}

#[macro_export]
macro_rules! assert_eq_event_logs {
    ($left:expr, $right:expr) => {{
        let left: Vec<String> = $left.iter().map(ToString::to_string).collect();
        let right: Vec<String> = $right.iter().map(ToString::to_string).collect();
        assert_eq!(left, right);
    }};
}

/// Assert that collection `a` contains collection `b`.
/// Checks that all elements in `b` are present in `a`.
///
/// # Examples
/// ```ignore
/// assert_a_contains_b!(a: all_logs, b: [expected_event1, expected_event2]);
/// ```
#[macro_export]
macro_rules! assert_a_contains_b {
    (a: $a:expr, b: $b:expr) => {{
        let a: Vec<String> = $a.iter().map(ToString::to_string).collect();
        let b: Vec<String> = $b.iter().map(ToString::to_string).collect();

        for expected_event in &b {
            if !a.contains(expected_event) {
                panic!(
                    "\n\nExpected event not found in 'a':\n{}\n\nActual event logs in 'a':\n{:#?}\n",
                    expected_event,
                    a
                );
            }
        }
    }};
}
