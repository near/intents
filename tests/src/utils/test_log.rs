// #[macro_export]
// macro_rules! assert_eq_event_logs {
//     ($left:expr, $right:expr) => {{
//         let left: Vec<String> = $left.iter().map(ToString::to_string).collect();
//         let right: Vec<String> = $right.iter().map(ToString::to_string).collect();
//         assert_eq!(left, right);
//     }};
// }

// /// Assert that collection `a` contains collection `b`.
// /// Checks that all elements in `b` are present in `a`.
// ///
// /// # Examples
// /// ```ignore
// /// assert_a_contains_b!(a: all_logs, b: [expected_event1, expected_event2]);
// /// ```
// #[macro_export]
// macro_rules! assert_a_contains_b {
//     (a: $a:expr, b: $b:expr) => {{
//         let a: Vec<String> = $a.iter().map(ToString::to_string).collect();
//         let b: Vec<String> = $b.iter().map(ToString::to_string).collect();

//         for expected_event in &b {
//             if !a.contains(expected_event) {
//                 panic!(
//                     "\n\nExpected event not found in 'a':\n{}\n\nActual event logs in 'a':\n{:#?}\n",
//                     expected_event,
//                     a
//                 );
//             }
//         }
//     }};
// }

// use near_sdk::Gas;
// use near_workspaces::result::ExecutionResult;

// #[allow(dead_code)]
// #[derive(Debug)]
// pub struct TestLog {
//     logs: Vec<String>,
//     receipt_failure_errors: Vec<String>,
//     gas_burnt_in_tx: Gas,
//     logs_and_gas_burnt_in_receipts: Vec<(Vec<String>, Gas)>,
// }

// impl From<ExecutionResult<near_workspaces::result::Value>> for TestLog {
//     fn from(outcome: ExecutionResult<near_workspaces::result::Value>) -> Self {
//         Self {
//             logs: outcome.logs().into_iter().map(str::to_string).collect(),
//             receipt_failure_errors: outcome
//                 .receipt_outcomes()
//                 .iter()
//                 .map(|s| {
//                     if let Err(e) = (*s).clone().into_result() {
//                         match e.into_inner() {
//                             Ok(o) => format!("OK: {o}"),
//                             Err(e) => format!("Err: {e}"),
//                         }
//                     } else {
//                         String::new()
//                     }
//                 })
//                 .collect::<Vec<_>>(),
//             gas_burnt_in_tx: outcome.total_gas_burnt,
//             logs_and_gas_burnt_in_receipts: outcome
//                 .receipt_outcomes()
//                 .iter()
//                 .map(|v| (v.logs.clone(), v.gas_burnt))
//                 .collect(),
//         }
//     }
// }

// impl TestLog {
//     pub fn logs(&self) -> &[String] {
//         &self.logs
//     }

//     pub const fn total_gas_burnt(&self) -> &Gas {
//         &self.gas_burnt_in_tx
//     }

//     pub const fn logs_and_gas_burnt_in_receipts(&self) -> &Vec<(Vec<String>, Gas)> {
//         &self.logs_and_gas_burnt_in_receipts
//     }
// }
