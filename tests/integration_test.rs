//! Integration tests for {{project-name}}.

#[test]
fn test_run_succeeds() {
    let result = {{crate_name}}::run();
    assert!(result.is_ok(), "run() should complete successfully");
}
