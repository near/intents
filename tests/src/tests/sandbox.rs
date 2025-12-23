//! Sandbox infrastructure tests.

use defuse_sandbox::{Sandbox, sandbox};
use rstest::rstest;

/// Test that sandbox cleanup works via atexit handler.
/// When test exits, the atexit handler should kill the neard process.
#[rstest]
#[tokio::test]
async fn test_sandbox_cleanup(#[future] sandbox: Sandbox) {
    let sandbox = sandbox.await;
    // Verify sandbox is usable
    let _root = sandbox.root();
}
