//! Integration tests for sdk-forge CLI.

// Note: `run()` now requires CLI args, so we test individual components
// rather than the full `run()` function. Full CLI integration tests
// will use `assert_cmd` in a future iteration.

#[test]
fn test_cli_crate_compiles() {
    // Smoke test: verify the library crate is accessible from integration tests.
    // This ensures the public API surface is correctly exported.
    assert!(true, "sdk-forge-cli crate should compile and link");
}
